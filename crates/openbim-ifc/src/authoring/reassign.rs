//! Changing an object's class in place (#483): [`AuthorOp::ReassignClass`].
//!
//! The entity keeps its id, file position and `GlobalId`; the overlay stages
//! the class change as a retype and the changed record as slot writes
//! (ADR 0010, 0022), so the change undoes exactly. The rules:
//!
//! 1. Both classes are an `IfcObject` and not an `IfcProject`; the target is
//!    declared and not abstract. Type objects are not reassigned (#636).
//! 2. Attributes move by name, case-insensitively. `$` and `*` are not
//!    carried; a slot the target derives gets `*`. A set attribute the target
//!    does not declare or derives is dropped, which is refused with
//!    `AttributesDropped` unless `drop_unmapped`, and then reported.
//! 3. `PredefinedType` and `ObjectType` move like any other attribute; the
//!    builder refuses a `PredefinedType` outside the new enumeration.
//! 4. The caller's `attributes` replace carried values. The record passes the
//!    checks of a created one; a `GlobalId` is claimed only if overridden.
//! 5. A transaction cannot shorten a record: a base entity whose new class
//!    declares fewer attributes than the record holds is refused with
//!    `UnsupportedReassignment` (#634). An entity the batch created is simply
//!    rebuilt.
//! 6. Every referrer is checked against the new class: its attribute must
//!    accept it, it must not fill an inverse of the old class the new one
//!    lacks, and every inverse the new class adds must stay within bounds.
//! 7. A typed object is refused when the type rule that checks its type
//!    (`CorrectTypeAssigned`, `CorrectStyleAssigned`) would change: the
//!    batch does not evaluate those rules.
//! 8. Neither the entity's nor its referrers' `WHERE` rules are evaluated:
//!    an `IfcRelSpaceBoundary` with `.PHYSICAL.` still accepts a wall moved
//!    to `IfcVirtualElement`.
//! 9. Ids, file order and `GlobalId` stay; the entity moves to the end of
//!    its new type's bucket.

use std::collections::BTreeSet;

use ifc_author::AuthorError;
use ifc_model::{EntityId, Value};
use ifc_schema::{EntityDef, Schema};

use super::check::{build, recheck, reference, relations};
use super::overlay::holds;
use super::plan::Planner;
use super::relate::TYPED;
use super::report::ReassignReport;
use super::{AuthoringFailure as F, NamedValues, ReassignClassOptions};
use crate::{attribute_slots, inverse_slots, AttributeSlot, InverseSlot};

/// The `WHERE` rule labels that check a typed object's class against its
/// type object.
const TYPE_RULES: [&str; 2] = ["CorrectTypeAssigned", "CorrectStyleAssigned"];

const GLOBAL_ID: &str = "GlobalId";

/// An inverse attribute with the index of its `FOR` attribute.
struct Inverse<'s> {
    slot: InverseSlot<'s>,
    for_index: usize,
}

impl Planner<'_, '_> {
    /// Change `id` to the class `type_name`; see the module docs.
    pub(super) fn reassign_class(
        &mut self,
        id: EntityId,
        type_name: &str,
        attributes: NamedValues,
        options: ReassignClassOptions,
    ) -> Result<ReassignReport, F> {
        let schema = self.schema;
        let old = self.view.get(id).cloned().ok_or(F::MissingEntity(id))?;
        let from_def = declared(schema, &old.type_name)?;
        let to_def = declared(schema, type_name)?;
        if to_def.abstract_ {
            return Err(F::AbstractEntity {
                type_name: to_def.name.clone(),
            });
        }
        let (from, to) = (from_def.name.as_str(), to_def.name.as_str());
        for class in [from, to] {
            if !schema.is_a(class, "IfcObject") || schema.is_a(class, "IfcProject") {
                return Err(F::NotA {
                    type_name: class.to_owned(),
                    expected: "IfcObject that is not an IfcProject".to_owned(),
                });
            }
        }
        let from_slots = slots(schema, from)?;
        let to_slots = slots(schema, to)?;
        if old.attributes.len() != from_slots.len() {
            return Err(F::Author(AuthorError::ArityMismatch {
                entity: old.type_name.to_string(),
                expected: from_slots.len(),
                found: old.attributes.len(),
            }));
        }
        let held = self.view.base().get(id).map_or(0, |e| e.attributes.len());
        if to_slots.len() < held {
            // TODO(#634)
            return Err(F::UnsupportedReassignment {
                entity: id,
                from: from.to_owned(),
                to: to.to_owned(),
                detail: format!(
                    "{to} declares {} attributes and the record holds {held}; \
                     a transaction cannot shorten a record",
                    to_slots.len()
                ),
            });
        }
        let overridden = attributes
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(GLOBAL_ID));
        let (values, dropped) = remap(&from_slots, &to_slots, &old.attributes, attributes);
        if !dropped.is_empty() && !options.drop_unmapped {
            return Err(F::AttributesDropped {
                entity: id,
                from: from.to_owned(),
                to: to.to_owned(),
                attributes: dropped,
            });
        }
        let entity = build(schema, to, &values)?;
        recheck(schema, &entity)?;
        relations(schema, &self.view, &entity, None)?;
        if overridden {
            self.claim_global_id(id, &entity)?;
        }
        self.view.replace(id, entity);
        self.check_referrers(id, from, to)?;
        self.check_type_rule(id, from, to)?;
        Ok(ReassignReport {
            entity: id,
            from: from.to_owned(),
            to: to.to_owned(),
            dropped: if options.drop_unmapped {
                dropped
            } else {
                Vec::new()
            },
        })
    }

    /// Every referrer of `id`, ascending, against the class `to` it now has
    /// (rule 6), then the inverses `to` adds.
    fn check_referrers(&self, id: EntityId, from: &str, to: &str) -> Result<(), F> {
        let schema = self.schema;
        let old = inverses(schema, from)?;
        let new = inverses(schema, to)?;
        // Per inverse of `to`, the referrers that fill it.
        let mut filled: Vec<BTreeSet<EntityId>> = vec![BTreeSet::new(); new.len()];
        for referrer in self.view.referrers(id) {
            let Some(entity) = self.view.get(referrer) else {
                continue;
            };
            declared(schema, &entity.type_name)?;
            let declared_attributes = schema.attributes(&entity.type_name);
            if declared_attributes.len() != entity.attributes.len() {
                return Err(F::Author(AuthorError::ArityMismatch {
                    entity: entity.type_name.to_string(),
                    expected: declared_attributes.len(),
                    found: entity.attributes.len(),
                }));
            }
            for (slot, (attribute, value)) in declared_attributes
                .iter()
                .zip(&entity.attributes)
                .enumerate()
            {
                if !holds(std::slice::from_ref(value), id) {
                    continue;
                }
                reference(schema, &self.view, &entity.type_name, attribute, id)?;
                let fills = |inverse: &Inverse<'_>| {
                    inverse.for_index == slot && schema.is_a(&entity.type_name, inverse.slot.entity)
                };
                let kept = new.iter().enumerate().filter(|(_, inverse)| fills(inverse));
                let mut any = false;
                for (position, _) in kept {
                    filled[position].insert(referrer);
                    any = true;
                }
                if !any {
                    if let Some(lost) = old.iter().find(|inverse| fills(inverse)) {
                        return Err(F::RelationshipLost {
                            entity: id,
                            relationship: referrer,
                            inverse: lost.slot.name.to_owned(),
                            to: to.to_owned(),
                        });
                    }
                }
            }
        }
        for (inverse, found) in new.iter().zip(&filled) {
            let unchanged = old.iter().any(|before| {
                before.slot.name == inverse.slot.name
                    && before.slot.entity == inverse.slot.entity
                    && before.slot.for_attribute == inverse.slot.for_attribute
                    && before.slot.aggregation == inverse.slot.aggregation
            });
            let (lower, upper) = (inverse.slot.lower(), inverse.slot.upper());
            let count = found.len() as u64;
            if !unchanged && (count < lower || upper.is_some_and(|upper| count > upper)) {
                return Err(F::InverseCardinality {
                    entity: id,
                    to: to.to_owned(),
                    inverse: inverse.slot.name.to_owned(),
                    found: found.len(),
                    lower,
                    upper,
                });
            }
        }
        Ok(())
    }

    /// A typed object keeps the type rule that checks its type (rule 7). A
    /// target class without a type rule accepts any existing type assignment,
    /// which is schema-valid; moving a type object with its occurrences is
    /// #636.
    fn check_type_rule(&self, id: EntityId, from: &str, to: &str) -> Result<(), F> {
        let Some(relationship) = self.related_by(&TYPED, id) else {
            return Ok(());
        };
        let (from_rule, to_rule) = (type_rule(self.schema, from), type_rule(self.schema, to));
        match to_rule {
            Some(to_rule) if from_rule.as_ref() != Some(&to_rule) => Err(F::TypeRuleChanged {
                object: id,
                type_object: self
                    .relating_of(&TYPED, relationship)
                    .unwrap_or(relationship),
                from_rule,
                to_rule,
            }),
            _ => Ok(()),
        }
    }
}

fn declared<'s>(schema: &'s Schema, type_name: &str) -> Result<&'s EntityDef, F> {
    schema.entity(type_name).ok_or_else(|| F::UnknownEntity {
        schema: schema.name().to_owned(),
        type_name: type_name.to_owned(),
    })
}

fn slots<'s>(schema: &'s Schema, type_name: &str) -> Result<Vec<AttributeSlot<'s>>, F> {
    attribute_slots(schema, type_name).map_err(|_| F::UnknownEntity {
        schema: schema.name().to_owned(),
        type_name: type_name.to_owned(),
    })
}

/// The inverses of `type_name`, each with its `FOR` attribute's index.
fn inverses<'s>(schema: &'s Schema, type_name: &str) -> Result<Vec<Inverse<'s>>, F> {
    let unknown = |type_name: &str| F::UnknownEntity {
        schema: schema.name().to_owned(),
        type_name: type_name.to_owned(),
    };
    inverse_slots(schema, type_name)
        .map_err(|_| unknown(type_name))?
        .into_iter()
        .map(|slot| {
            let for_index = schema
                .entity(slot.entity)
                .and_then(|_| {
                    schema
                        .attributes(slot.entity)
                        .iter()
                        .position(|a| a.name.eq_ignore_ascii_case(slot.for_attribute))
                })
                .ok_or_else(|| unknown(slot.entity))?;
            Ok(Inverse { slot, for_index })
        })
        .collect()
}

/// The values the new class starts from: those of the old record by name,
/// under the caller's, and the set attributes of the old class that are lost.
fn remap(
    from: &[AttributeSlot<'_>],
    to: &[AttributeSlot<'_>],
    record: &[Value],
    overrides: NamedValues,
) -> (NamedValues, Vec<String>) {
    let mut values: NamedValues = Vec::new();
    let mut dropped = Vec::new();
    for slot in from {
        let value = record.get(slot.index).unwrap_or(&Value::Null);
        if matches!(value, Value::Null | Value::Derived) {
            continue;
        }
        let target = to
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(slot.name));
        match target {
            Some(target) if !target.derived => {
                if !overrides
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case(target.name))
                {
                    values.push((target.name.to_owned(), value.clone()));
                }
            }
            _ => dropped.push(slot.name.to_owned()),
        }
    }
    values.extend(overrides);
    (values, dropped)
}

/// `Owner.Label` of the nearest class, `type_name` first, that declares a
/// rule checking the type a typed object has.
fn type_rule(schema: &Schema, type_name: &str) -> Option<String> {
    std::iter::once(type_name)
        .chain(schema.supertypes(type_name))
        .filter_map(|name| schema.entity(name))
        .find_map(|definition| {
            definition
                .where_rules
                .iter()
                .find(|rule| TYPE_RULES.contains(&rule.label.as_str()))
                .map(|rule| format!("{}.{}", definition.name, rule.label))
        })
}

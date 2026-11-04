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
//! 5. A class that declares fewer attributes than the stored record shortens
//!    it: the overlay stages the shorter record as a truncation, applied
//!    after the edits, so undo, redo and replay stay exact (#634).
//! 6. Every referrer is checked against the new class: its attribute must
//!    accept it, it must not fill an inverse of the old class the new one
//!    lacks, and every inverse the new class adds must stay within bounds.
//! 7. A typed object is refused when the type rule that checks its type
//!    (`CorrectTypeAssigned`, `CorrectStyleAssigned`) would change: the
//!    batch does not evaluate those rules.
//!    With `remove_relationships` the entity is instead taken out of the
//!    `IfcRelationship`s that fail rule 6 (not its bounds) or this rule, as
//!    `AuthorOp::Remove` detaches (#635). A removed relationship must not
//!    leave another entity below a mandatory inverse; any other referrer is
//!    still refused (`referrers`).
//! 8. Neither the entity's nor its referrers' `WHERE` rules are evaluated:
//!    an `IfcRelSpaceBoundary` with `.PHYSICAL.` still accepts a wall moved
//!    to `IfcVirtualElement`.
//! 9. Ids, file order and `GlobalId` stay; the entity moves to the end of
//!    its new type's bucket.

mod referrers;

use ifc_author::AuthorError;
use ifc_model::{EntityId, Value};
use ifc_schema::{EntityDef, Schema};

use super::check::{build, recheck, relations};
use super::plan::Planner;
use super::relate::TYPED;
use super::report::{ReassignReport, RelationshipConflict};
use super::{AuthoringFailure as F, NamedValues, ReassignClassOptions};
use crate::{attribute_slots, AttributeSlot};

/// The `WHERE` rule labels that check a typed object's class against its
/// type object.
const TYPE_RULES: [&str; 2] = ["CorrectTypeAssigned", "CorrectStyleAssigned"];

const GLOBAL_ID: &str = "GlobalId";

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
        let mut removed = if options.remove_relationships {
            self.detach_unholdable(id, from, to)?
        } else {
            if let Some(first) = self.unholdable(id, from, to, true)?.into_iter().next() {
                return Err(first.refusal);
            }
            Vec::new()
        };
        while let Some(conflict) = self.type_conflict(id, from, to) {
            if !options.remove_relationships {
                return Err(conflict.refusal);
            }
            removed.push(self.leave_relationship(id, conflict.relationship, conflict.conflict)?);
        }
        self.check_inverse_bounds(id, from, to)?;
        removed.sort_by_key(|entry| entry.relationship);
        Ok(ReassignReport {
            entity: id,
            from: from.to_owned(),
            to: to.to_owned(),
            dropped: if options.drop_unmapped {
                dropped
            } else {
                Vec::new()
            },
            removed_relationships: removed,
        })
    }

    /// The type rule a typed object would fall under instead of the one that
    /// checks its type (rule 7). A target class without a type rule accepts
    /// any existing type assignment, which is schema-valid; moving a type
    /// object with its occurrences is #636.
    fn type_conflict(&self, id: EntityId, from: &str, to: &str) -> Option<TypeConflict> {
        let relationship = self.related_by(&TYPED, id)?;
        let (from_rule, to_rule) = (type_rule(self.schema, from), type_rule(self.schema, to));
        let to_rule = to_rule.filter(|to_rule| from_rule.as_ref() != Some(to_rule))?;
        let type_object = self
            .relating_of(&TYPED, relationship)
            .unwrap_or(relationship);
        Some(TypeConflict {
            relationship,
            conflict: RelationshipConflict::TypeRule {
                type_object,
                to_rule: to_rule.clone(),
            },
            refusal: F::TypeRuleChanged {
                object: id,
                type_object,
                from_rule,
                to_rule,
            },
        })
    }
}

/// The relationship that assigns the type a typed object falls under another
/// rule than.
struct TypeConflict {
    relationship: EntityId,
    conflict: RelationshipConflict,
    refusal: F,
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

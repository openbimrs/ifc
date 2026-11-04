//! The referrers of a reassigned entity (#483, #635): which ones the new
//! class cannot hold, and the relationships a reassignment may take it out
//! of.

use std::collections::BTreeSet;

use ifc_author::AuthorError;
use ifc_model::{EntityId, Value};
use ifc_schema::Schema;

use super::{declared, Planner};
use crate::authoring::check::reference;
use crate::authoring::overlay::holds;
use crate::authoring::report::{RelationshipConflict, RemovedRelationship};
use crate::authoring::AuthoringFailure as F;
use crate::{inverse_slots, InverseSlot};

/// An inverse attribute with the index of its `FOR` attribute.
pub(super) struct Inverse<'s> {
    slot: InverseSlot<'s>,
    for_index: usize,
}

/// A referrer the new class cannot hold, with the refusal it earns.
pub(super) struct Unholdable {
    pub(super) referrer: EntityId,
    pub(super) conflict: RelationshipConflict,
    pub(super) refusal: F,
}

impl Planner<'_, '_> {
    /// For each referrer of `id` that the class `to` cannot hold, the first
    /// problem (rule 6), ascending by referrer: its attribute rejects `to`,
    /// or it fills an inverse of `from` that `to` lacks. With `first_only` the
    /// scan stops at the first problem, so a malformed later referrer cannot
    /// replace its refusal.
    pub(super) fn unholdable(
        &self,
        id: EntityId,
        from: &str,
        to: &str,
        first_only: bool,
    ) -> Result<Vec<Unholdable>, F> {
        let schema = self.schema;
        let old = inverses(schema, from)?;
        let new = inverses(schema, to)?;
        let mut found = Vec::new();
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
                match reference(schema, &self.view, &entity.type_name, attribute, id) {
                    Ok(()) => {}
                    Err(F::WrongReferenceType {
                        entity: owner,
                        attribute,
                        target,
                        actual,
                        expected,
                    }) => {
                        found.push(Unholdable {
                            referrer,
                            conflict: RelationshipConflict::WrongReferenceType {
                                attribute: attribute.clone(),
                                expected: expected.clone(),
                            },
                            refusal: F::WrongReferenceType {
                                entity: owner,
                                attribute,
                                target,
                                actual,
                                expected,
                            },
                        });
                        if first_only {
                            return Ok(found);
                        }
                        break;
                    }
                    Err(other) => return Err(other),
                }
                let fills = |inverse: &Inverse<'_>| {
                    inverse.for_index == slot && schema.is_a(&entity.type_name, inverse.slot.entity)
                };
                if !new.iter().any(fills) {
                    if let Some(lost) = old.iter().find(|inverse| fills(inverse)) {
                        let inverse = lost.slot.name.to_owned();
                        found.push(Unholdable {
                            referrer,
                            conflict: RelationshipConflict::Lost {
                                inverse: inverse.clone(),
                            },
                            refusal: F::RelationshipLost {
                                entity: id,
                                relationship: referrer,
                                inverse,
                                to: to.to_owned(),
                            },
                        });
                        if first_only {
                            return Ok(found);
                        }
                        break;
                    }
                }
            }
        }
        Ok(found)
    }

    /// Every inverse the class `to` adds stays within its bounds (rule 6).
    pub(super) fn check_inverse_bounds(&self, id: EntityId, from: &str, to: &str) -> Result<(), F> {
        let schema = self.schema;
        let old = inverses(schema, from)?;
        let new = inverses(schema, to)?;
        // Per inverse of `to`, the referrers that fill it.
        let mut filled: Vec<BTreeSet<EntityId>> = vec![BTreeSet::new(); new.len()];
        for referrer in self.view.referrers(id) {
            let Some(entity) = self.view.get(referrer) else {
                continue;
            };
            for (slot, value) in entity.attributes.iter().enumerate() {
                if !holds(std::slice::from_ref(value), id) {
                    continue;
                }
                for (position, inverse) in new.iter().enumerate() {
                    if inverse.for_index == slot
                        && schema.is_a(&entity.type_name, inverse.slot.entity)
                    {
                        filled[position].insert(referrer);
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

    /// Take `id` out of every relationship the class `to` cannot hold
    /// (`remove_relationships`). A referrer that is not an `IfcRelationship`
    /// still earns its refusal.
    pub(super) fn detach_unholdable(
        &mut self,
        id: EntityId,
        from: &str,
        to: &str,
    ) -> Result<Vec<RemovedRelationship>, F> {
        let mut taken = Vec::new();
        for unholdable in self.unholdable(id, from, to, false)? {
            let relationship = unholdable.referrer;
            if !self.is_relationship(relationship) {
                return Err(unholdable.refusal);
            }
            taken.push(self.leave_relationship(id, relationship, unholdable.conflict)?);
        }
        Ok(taken)
    }

    /// Take `id` out of `relationship`, refusing when its removal leaves
    /// another entity below a mandatory inverse.
    pub(super) fn leave_relationship(
        &mut self,
        id: EntityId,
        relationship: EntityId,
        reason: RelationshipConflict,
    ) -> Result<RemovedRelationship, F> {
        let before = self
            .view
            .get(relationship)
            .cloned()
            .ok_or(F::MissingEntity(relationship))?;
        let type_name = declared(self.schema, &before.type_name)?.name.clone();
        let removed = self.detach(id, relationship)?;
        if removed {
            let mut ends = Vec::new();
            references(&before.attributes, &mut ends);
            for end in ends.into_iter().filter(|end| *end != id) {
                self.require_filled(relationship, &before, end)?;
            }
        }
        Ok(RemovedRelationship {
            relationship,
            object: id,
            type_name,
            removed,
            reason,
        })
    }

    fn is_relationship(&self, id: EntityId) -> bool {
        self.view
            .get(id)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, "IfcRelationship"))
    }

    /// `end` keeps the filler of each mandatory inverse the removed
    /// `relationship` (as it was in `before`) filled, unless it had none to
    /// spare.
    fn require_filled(
        &self,
        relationship: EntityId,
        before: &ifc_model::Entity,
        end: EntityId,
    ) -> Result<(), F> {
        let schema = self.schema;
        let Some(entity) = self.view.get(end) else {
            return Ok(());
        };
        for inverse in inverses(schema, &entity.type_name)? {
            let lower = inverse.slot.lower();
            let filled_by = |holder: &ifc_model::Entity| {
                schema.is_a(&holder.type_name, inverse.slot.entity)
                    && holder
                        .attributes
                        .get(inverse.for_index)
                        .is_some_and(|value| holds(std::slice::from_ref(value), end))
            };
            if lower == 0 || !filled_by(before) {
                continue;
            }
            let after = self
                .view
                .referrers(end)
                .into_iter()
                .filter_map(|holder| self.view.get(holder))
                .filter(|holder| filled_by(holder))
                .count() as u64;
            // It had `after + 1` fillers with the relationship.
            if after + 1 == lower {
                return Err(F::RelationshipRequired {
                    relationship,
                    entity: end,
                    inverse: inverse.slot.name.to_owned(),
                    lower,
                });
            }
        }
        Ok(())
    }
}

/// Every entity `values` reference, at any depth, in order.
fn references(values: &[Value], found: &mut Vec<EntityId>) {
    for value in values {
        match value {
            Value::Ref(id) if !found.contains(id) => found.push(*id),
            Value::List(items) => references(items, found),
            Value::Typed { value, .. } => references(std::slice::from_ref(value), found),
            _ => {}
        }
    }
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

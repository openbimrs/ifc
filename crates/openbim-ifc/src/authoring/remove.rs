//! Removing an entity together with its relationships.
//!
//! `Model::remove` leaves every reference to the entity dangling, and a
//! transaction refuses that. An objectified relationship (`IfcRelationship`)
//! is the one kind of referrer whose meaning survives losing an end: the
//! entity is taken out of the relationship's aggregate, and a relationship
//! left relating nothing, or whose single-valued end it was, is removed with
//! it. Any other referrer, such as a placement another placement is
//! relative to, still needs the entity, so the removal is refused and the
//! caller decides.

use ifc_model::{EntityId, Value};

use super::overlay::holds;
use super::plan::Planner;
use super::AuthoringFailure as F;

impl Planner<'_, '_> {
    pub(super) fn remove(&mut self, target: EntityId) -> Result<(), F> {
        if self.view.get(target).is_none() {
            return Err(F::MissingEntity(target));
        }
        for referrer in self.view.referrers(target) {
            self.detach(target, referrer)?;
        }
        self.view.remove(target);
        Ok(())
    }

    /// Take `target` out of the relationship `referrer`: out of its
    /// aggregate, or the relationship removed when that leaves it relating
    /// nothing or `target` was a single-valued end. Whether the
    /// relationship was removed.
    pub(super) fn detach(&mut self, target: EntityId, referrer: EntityId) -> Result<bool, F> {
        let Some(mut entity) = self.view.get(referrer).cloned() else {
            return Ok(false);
        };
        if !self.schema.is_a(&entity.type_name, "IfcRelationship") {
            return Err(F::StillReferenced {
                id: target,
                by: referrer,
            });
        }
        let mut loses_an_end = false;
        for value in &mut entity.attributes {
            match value {
                Value::Ref(id) if *id == target => loses_an_end = true,
                Value::List(items) if items.contains(&Value::Ref(target)) => {
                    items.retain(|item| *item != Value::Ref(target));
                    loses_an_end |= items.is_empty() || holds(items, target);
                }
                other => loses_an_end |= holds(std::slice::from_ref(other), target),
            }
        }
        if loses_an_end {
            if let Some(by) = self
                .view
                .referrers(referrer)
                .into_iter()
                .find(|by| *by != target)
            {
                return Err(F::StillReferenced { id: referrer, by });
            }
            self.view.remove(referrer);
        } else {
            self.view.replace(referrer, entity);
        }
        Ok(loses_an_end)
    }
}

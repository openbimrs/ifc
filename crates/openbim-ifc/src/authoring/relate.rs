//! The three relationships the builders write, each refusing an object
//! the schema lets hold it only once.
//!
//! `IfcElement.ContainedInStructure`, `IfcObjectDefinition.Decomposes` and
//! (IFC4 on) `IfcObject.IsTypedBy` are inverse `SET [0:1]`s, and IFC2X3's
//! `IfcObject` rule `WR1` allows one `IfcRelDefinesByType`; a second
//! relationship of the kind would make the file invalid, so it is refused
//! here, not written, unless the caller asks to replace it: the object
//! then leaves the relationship it has (which goes when it is left
//! relating nothing) and joins the new one.

use ifc_model::{EntityId, Value};

use super::plan::{with_owner, Planner};
use super::{AssignTypeOptions, AuthoringFailure as F};

/// A relationship type with its two ends, by attribute name.
pub(super) struct Relation {
    type_name: &'static str,
    entity: &'static str,
    relating: &'static str,
    related: &'static str,
}

const CONTAINED: Relation = Relation {
    type_name: "IFCRELCONTAINEDINSPATIALSTRUCTURE",
    entity: "IfcRelContainedInSpatialStructure",
    relating: "RelatingStructure",
    related: "RelatedElements",
};

const AGGREGATES: Relation = Relation {
    type_name: "IFCRELAGGREGATES",
    entity: "IfcRelAggregates",
    relating: "RelatingObject",
    related: "RelatedObjects",
};

pub(super) const TYPED: Relation = Relation {
    type_name: "IFCRELDEFINESBYTYPE",
    entity: "IfcRelDefinesByType",
    relating: "RelatingType",
    related: "RelatedObjects",
};

impl Planner<'_, '_> {
    pub(super) fn contain(
        &mut self,
        structure: EntityId,
        elements: &[EntityId],
        owner: Option<EntityId>,
        replace: bool,
    ) -> Result<EntityId, F> {
        self.relate(&CONTAINED, structure, elements, owner, replace)
    }

    pub(super) fn aggregate(
        &mut self,
        parent: EntityId,
        parts: &[EntityId],
        owner: Option<EntityId>,
        replace: bool,
    ) -> Result<EntityId, F> {
        self.relate(&AGGREGATES, parent, parts, owner, replace)
    }

    /// Type `objects` by `type_object`, then propagate what `options`
    /// asks (see `propagate`).
    pub(super) fn assign_type(
        &mut self,
        type_object: EntityId,
        objects: &[EntityId],
        owner: Option<EntityId>,
        options: &AssignTypeOptions,
    ) -> Result<EntityId, F> {
        // The previous type of each object, read before the relationship
        // that states it is detached.
        let mut previous = Vec::new();
        if options.replace {
            for object in objects {
                if let Some(relationship) = self.related_by(&TYPED, *object) {
                    if let Some(old) = self.relating_of(&TYPED, relationship) {
                        previous.push((*object, old));
                    }
                }
            }
        }
        let relationship = self.relate(&TYPED, type_object, objects, owner, options.replace)?;
        for (object, old) in previous {
            self.strip_type(old, object)?;
        }
        self.propagate(type_object, objects, owner, options)?;
        Ok(relationship)
    }

    fn relate(
        &mut self,
        relation: &Relation,
        relating: EntityId,
        related: &[EntityId],
        owner: Option<EntityId>,
        replace: bool,
    ) -> Result<EntityId, F> {
        if related.is_empty() {
            return Err(F::InvalidRelationship {
                relationship: relation.type_name,
                detail: "relates nothing",
            });
        }
        if related.contains(&relating) {
            return Err(F::InvalidRelationship {
                relationship: relation.type_name,
                detail: "relates an entity to itself",
            });
        }
        for object in related {
            if let Some(existing) = self.related_by(relation, *object) {
                if !replace {
                    return Err(F::AlreadyRelated {
                        id: *object,
                        relationship: relation.type_name,
                        existing,
                    });
                }
                self.leave(existing, relation.related, *object)?;
            }
        }
        let attributes = vec![
            (relation.relating.to_owned(), Value::Ref(relating)),
            (
                relation.related.to_owned(),
                Value::List(related.iter().copied().map(Value::Ref).collect()),
            ),
        ];
        self.create(relation.entity, with_owner(attributes, owner))
    }

    /// The relating end of `relationship`, of `relation`'s type.
    pub(super) fn relating_of(
        &self,
        relation: &Relation,
        relationship: EntityId,
    ) -> Option<EntityId> {
        let slot = self
            .schema
            .attributes(relation.entity)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(relation.relating))?;
        self.view
            .get(relationship)?
            .attributes
            .get(slot)?
            .as_ref_id()
    }

    /// Take `object` out of the list `attribute` of `relationship`; a
    /// relationship left relating nothing is removed.
    pub(super) fn leave(
        &mut self,
        relationship: EntityId,
        attribute: &str,
        object: EntityId,
    ) -> Result<(), F> {
        let entity = self
            .view
            .get(relationship)
            .ok_or(F::MissingEntity(relationship))?;
        let slot = self
            .schema
            .attributes(&entity.type_name)
            .iter()
            .position(|declared| declared.name.eq_ignore_ascii_case(attribute));
        let rest: Vec<Value> = match slot.and_then(|slot| entity.attributes.get(slot)) {
            Some(Value::List(items)) => items
                .iter()
                .filter(|item| **item != Value::Ref(object))
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        if rest.is_empty() {
            self.remove(relationship)
        } else {
            self.edit(
                relationship,
                vec![(attribute.to_owned(), Value::List(rest))],
            )
            .map(drop)
        }
    }

    /// The relationship of `relation`'s type already relating `object`.
    pub(super) fn related_by(&self, relation: &Relation, object: EntityId) -> Option<EntityId> {
        let slot = self
            .schema
            .attributes(relation.entity)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(relation.related))?;
        self.view
            .ids_of_type(relation.type_name)
            .into_iter()
            .find(|id| {
                self.view
                    .get(*id)
                    .and_then(|entity| entity.attributes.get(slot))
                    .is_some_and(|value| match value {
                        Value::List(items) => items.contains(&Value::Ref(object)),
                        Value::Ref(id) => *id == object,
                        _ => false,
                    })
            })
    }
}

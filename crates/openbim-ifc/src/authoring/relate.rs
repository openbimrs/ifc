//! The three relationships the builders write, each refusing an object
//! the schema lets hold it only once.
//!
//! `IfcElement.ContainedInStructure`, `IfcObjectDefinition.Decomposes` and
//! (IFC4 on) `IfcObject.IsTypedBy` are inverse `SET [0:1]`s, and IFC2X3's
//! `IfcObject` rule `WR1` allows one `IfcRelDefinesByType`; a second
//! relationship of the kind would make the file invalid, so it is refused
//! here, not written.

use ifc_model::{EntityId, Value};

use super::plan::{with_owner, Planner};
use super::AuthoringFailure as F;

/// A relationship type with its two ends, by attribute name.
struct Relation {
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

const TYPED: Relation = Relation {
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
    ) -> Result<EntityId, F> {
        self.relate(&CONTAINED, structure, elements, owner)
    }

    pub(super) fn aggregate(
        &mut self,
        parent: EntityId,
        parts: &[EntityId],
        owner: Option<EntityId>,
    ) -> Result<EntityId, F> {
        self.relate(&AGGREGATES, parent, parts, owner)
    }

    pub(super) fn assign_type(
        &mut self,
        type_object: EntityId,
        objects: &[EntityId],
        owner: Option<EntityId>,
    ) -> Result<EntityId, F> {
        self.relate(&TYPED, type_object, objects, owner)
    }

    fn relate(
        &mut self,
        relation: &Relation,
        relating: EntityId,
        related: &[EntityId],
        owner: Option<EntityId>,
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
                return Err(F::AlreadyRelated {
                    id: *object,
                    relationship: relation.type_name,
                    existing,
                });
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

    /// The relationship of `relation`'s type already relating `object`.
    fn related_by(&self, relation: &Relation, object: EntityId) -> Option<EntityId> {
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

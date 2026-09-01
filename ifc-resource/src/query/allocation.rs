//! `IfcRelAssignsToResource` projections and inverse lookup.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::view::{validate_object_assignment, Record, ResourceView};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAllocation {
    relation: EntityId,
    resource: EntityId,
    related_objects: Vec<EntityId>,
    related_objects_type: Option<String>,
}

impl ResourceAllocation {
    #[must_use]
    pub fn relation_id(&self) -> EntityId {
        self.relation
    }

    #[must_use]
    pub fn resource_id(&self) -> EntityId {
        self.resource
    }

    #[must_use]
    pub fn related_objects(&self) -> &[EntityId] {
        &self.related_objects
    }

    #[must_use]
    pub fn related_objects_type(&self) -> Option<&str> {
        self.related_objects_type.as_deref()
    }
}

impl<'m, 's> ResourceView<'m, 's> {
    pub fn allocation(&self, id: EntityId) -> ResourceResult<ResourceAllocation> {
        let record = self.record(id, "IfcRelAssignsToResource")?;
        decode_allocation(record)
    }

    pub fn allocations_for(&self, resource: EntityId) -> ResourceResult<Vec<ResourceAllocation>> {
        self.resource(resource)?;
        let mut result = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelAssignsToResource") {
            let allocation = self.allocation(relation)?;
            if allocation.resource == resource {
                result.push(allocation);
            }
        }
        Ok(result)
    }
}

fn decode_allocation(record: Record<'_, '_>) -> ResourceResult<ResourceAllocation> {
    let resource = record.required_ref_select(
        "RelatingResource",
        "IfcResourceSelect",
        &["IfcResource", "IfcTypeResource"],
    )?;
    let related_objects = record.refs("RelatedObjects", "IfcObjectDefinition", 1, false, true)?;
    let related_objects_type = record.optional_enum("RelatedObjectsType")?;
    validate_object_assignment(
        record.model,
        record.schema,
        Some(record.id),
        related_objects_type,
        &related_objects,
    )?;
    if related_objects.contains(&resource) {
        return Err(ResourceError::SemanticViolation {
            entity: Some(record.id),
            rule: "IfcRelAssignsToResource must not assign its resource to itself",
        });
    }
    Ok(ResourceAllocation {
        relation: record.id,
        resource,
        related_objects,
        related_objects_type: related_objects_type.map(str::to_owned),
    })
}

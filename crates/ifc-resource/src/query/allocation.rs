//! `IfcRelAssignsToResource` projections and inverse lookup.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::view::{validate_object_assignment, Record, ResourceView};
use crate::ResourceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Decoded `IfcRelAssignsToResource` assignment: a resource and the objects
/// assigned to it.
pub struct ResourceAllocation {
    relation: EntityId,
    resource: EntityId,
    related_objects: Vec<EntityId>,
    related_objects_type: Option<String>,
}

impl ResourceAllocation {
    /// The entity id of the `IfcRelAssignsToResource` relation itself.
    #[must_use]
    pub fn relation_id(&self) -> EntityId {
        self.relation
    }

    /// The `RelatingResource` attribute: the assigned resource or resource
    /// type.
    #[must_use]
    pub fn resource_id(&self) -> EntityId {
        self.resource
    }

    /// The `RelatedObjects` attribute: objects the resource is assigned to.
    #[must_use]
    pub fn related_objects(&self) -> &[EntityId] {
        &self.related_objects
    }

    /// The `RelatedObjectsType` enumeration, when authored.
    #[must_use]
    pub fn related_objects_type(&self) -> Option<&str> {
        self.related_objects_type.as_deref()
    }
}

impl<'m, 's> ResourceView<'m, 's> {
    /// Projects an `IfcRelAssignsToResource` relation by entity id.
    pub fn allocation(&self, id: EntityId) -> ResourceResult<ResourceAllocation> {
        let record = self.record(id, "IfcRelAssignsToResource")?;
        decode_allocation(record)
    }

    /// Every `IfcRelAssignsToResource` relation naming this resource as
    /// `RelatingResource`, in ancestor order.
    ///
    /// Under IFC2X3 an `IfcConstructionMaterialResource` or
    /// `IfcConstructionProductResource` also carries the release's `WR1`
    /// (at most one such relation) and `WR2` (its `RelatedObjectsType`, when
    /// authored, is `PRODUCT`); a violation is a typed refusal.
    pub fn allocations_for(&self, resource: EntityId) -> ResourceResult<Vec<ResourceAllocation>> {
        let projected = self.resource(resource)?;
        let mut result = Vec::new();
        for relation in self.ids_of_ancestor("IfcRelAssignsToResource") {
            let allocation = self.allocation(relation)?;
            if allocation.resource == resource {
                result.push(allocation);
            }
        }
        if self.is_ifc2x3()
            && matches!(projected.kind(), ResourceKind::Material | ResourceKind::Product)
        {
            if result.len() > 1 {
                return Err(ResourceError::SemanticViolation {
                    entity: Some(resource),
                    rule: "IFC2X3 material/product resource WR1: at most one ResourceOf",
                });
            }
            if result.first().is_some_and(|allocation| {
                allocation
                    .related_objects_type()
                    .is_some_and(|category| !category.eq_ignore_ascii_case("PRODUCT"))
            }) {
                return Err(ResourceError::SemanticViolation {
                    entity: Some(resource),
                    rule: "IFC2X3 material/product resource WR2: ResourceOf must be PRODUCT",
                });
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

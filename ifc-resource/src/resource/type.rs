//! IFC4 `IfcConstructionResourceType` occurrence-kind projections.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::view::Record;

/// Concrete `IfcConstructionResourceType` specialization, matching
/// `ResourceKind` but for the type-level (catalog) entity rather than the
/// occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceTypeKind {
    Labor,
    Equipment,
    Crew,
    Material,
    Product,
    Subcontract,
}

impl ResourceTypeKind {
    fn from_type(type_name: &str) -> Option<Self> {
        if type_name.eq_ignore_ascii_case("IfcLaborResourceType") {
            Some(Self::Labor)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionEquipmentResourceType") {
            Some(Self::Equipment)
        } else if type_name.eq_ignore_ascii_case("IfcCrewResourceType") {
            Some(Self::Crew)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionMaterialResourceType") {
            Some(Self::Material)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionProductResourceType") {
            Some(Self::Product)
        } else if type_name.eq_ignore_ascii_case("IfcSubContractResourceType") {
            Some(Self::Subcontract)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConstructionResourceType<'m, 's> {
    record: Record<'m, 's>,
    kind: ResourceTypeKind,
}

impl<'m, 's> ConstructionResourceType<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let kind = ResourceTypeKind::from_type(&record.entity.type_name).ok_or_else(|| {
            ResourceError::WrongType {
                id: record.id,
                expected: "concrete IfcConstructionResourceType occurrence",
                actual: record.entity.type_name.to_string(),
            }
        })?;
        Ok(Self { record, kind })
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    pub fn kind(&self) -> ResourceTypeKind {
        self.kind
    }

    pub fn name(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Name")
    }

    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Identification")
    }

    pub fn long_description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LongDescription")
    }

    pub fn resource_type(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ResourceType")
    }

    /// Enforces the shared `CorrectPredefinedType` WHERE rule across all six
    /// concrete IFC4 resource-type entities: USERDEFINED requires the
    /// inherited `IfcTypeResource.ResourceType` label.
    pub fn predefined_type(&self) -> ResourceResult<Option<&'m str>> {
        let value = self.record.optional_enum("PredefinedType")?;
        if value.is_some_and(|value| value.eq_ignore_ascii_case("USERDEFINED"))
            && self
                .record
                .optional_text("ResourceType")?
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(ResourceError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "CorrectPredefinedType requires ResourceType for USERDEFINED",
            });
        }
        Ok(value)
    }

    pub fn base_costs(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("BaseCosts", "IfcAppliedValue", 1, true, false)
    }

    pub fn base_quantity(&self) -> ResourceResult<Option<EntityId>> {
        self.record
            .optional_ref("BaseQuantity", "IfcPhysicalQuantity")
    }
}

impl<'m, 's> crate::view::ResourceView<'m, 's> {
    pub fn resource_type(&self, id: EntityId) -> ResourceResult<ConstructionResourceType<'m, 's>> {
        ConstructionResourceType::from_record(self.record(id, "IfcConstructionResourceType")?)
    }

    /// Resolves the `IfcConstructionResourceType` assigned to a construction
    /// resource occurrence through `IfcRelDefinesByType`, if any.
    ///
    /// `IfcTypeObject.Types` is an inverse `SET [0:1]`: an occurrence may be
    /// typed by at most one relation. A second authored relation naming the
    /// same occurrence is a modeling defect, reported as a typed refusal
    /// rather than silently picking the first or last match.
    pub fn assigned_resource_type(&self, occurrence: EntityId) -> ResourceResult<Option<EntityId>> {
        self.resource(occurrence)?;
        let mut found = None;
        for relation in self.ids_of_ancestor("IfcRelDefinesByType") {
            let record = self.record(relation, "IfcRelDefinesByType")?;
            let related = record.refs("RelatedObjects", "IfcObject", 1, false, true)?;
            if !related.contains(&occurrence) {
                continue;
            }
            let relating_type =
                record.required_ref("RelatingType", "IfcConstructionResourceType")?;
            if let Some(existing) = found.replace(relating_type) {
                if existing != relating_type {
                    return Err(ResourceError::SemanticViolation {
                        entity: Some(occurrence),
                        rule: "IfcTypeObject.Types permits at most one type assignment",
                    });
                }
            }
        }
        Ok(found)
    }
}

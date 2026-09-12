//! IFC4 construction-resource occurrence projections.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::usage::ResourceTime;
use crate::view::Record;

/// Concrete `IfcConstructionResource` specialization, one of the six IFC4
/// resource-occurrence entities this crate projects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceKind {
    /// `IfcLaborResource`: a person or crew role.
    Labor,
    /// `IfcConstructionEquipmentResource`: machinery or tooling.
    Equipment,
    /// `IfcCrewResource`: a composed group of resources.
    Crew,
    /// `IfcConstructionMaterialResource`: a consumed material.
    Material,
    /// `IfcConstructionProductResource`: a resource realized as a product.
    Product,
    /// `IfcSubContractResource`: work delegated to a subcontractor.
    Subcontract,
}

impl ResourceKind {
    fn from_type(type_name: &str) -> Option<Self> {
        if type_name.eq_ignore_ascii_case("IfcLaborResource") {
            Some(Self::Labor)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionEquipmentResource") {
            Some(Self::Equipment)
        } else if type_name.eq_ignore_ascii_case("IfcCrewResource") {
            Some(Self::Crew)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionMaterialResource") {
            Some(Self::Material)
        } else if type_name.eq_ignore_ascii_case("IfcConstructionProductResource") {
            Some(Self::Product)
        } else if type_name.eq_ignore_ascii_case("IfcSubContractResource") {
            Some(Self::Subcontract)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Borrowed projection of a concrete `IfcConstructionResource` occurrence.
pub struct ConstructionResource<'m, 's> {
    record: Record<'m, 's>,
    kind: ResourceKind,
}

impl<'m, 's> ConstructionResource<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let kind = ResourceKind::from_type(&record.entity.type_name).ok_or_else(|| {
            ResourceError::WrongType {
                id: record.id,
                expected: "concrete IfcConstructionResource occurrence",
                actual: record.entity.type_name.to_string(),
            }
        })?;
        Ok(Self { record, kind })
    }

    /// The entity id of the projected occurrence.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// Which concrete `IfcConstructionResource` subtype this occurrence is.
    #[must_use]
    pub fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `Identification` attribute, when authored.
    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Identification")
    }

    /// The `LongDescription` attribute, when authored.
    pub fn long_description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LongDescription")
    }

    /// The `PredefinedType` enumeration, when authored.
    ///
    /// A `USERDEFINED` value requires `ObjectType` to be set; that
    /// constraint is enforced here rather than left to the caller.
    pub fn predefined_type(&self) -> ResourceResult<Option<&'m str>> {
        let value = self.record.optional_enum("PredefinedType")?;
        self.record.require_object_type_if(
            value.is_some_and(|value| value.eq_ignore_ascii_case("USERDEFINED")),
            "USERDEFINED resource PredefinedType requires ObjectType",
        )?;
        Ok(value)
    }

    /// The `Usage` attribute: authored `IfcResourceTime`, when present.
    pub fn usage(&self) -> ResourceResult<Option<ResourceTime<'m, 's>>> {
        self.record
            .optional_ref("Usage", "IfcResourceTime")?
            .map(|id| Record::new(self.record.model, self.record.schema, id, "IfcResourceTime"))
            .transpose()
            .map(|record| record.map(ResourceTime::from_record))
    }

    /// The `BaseCosts` attribute: applied-value references, when authored.
    pub fn base_costs(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("BaseCosts", "IfcAppliedValue", 1, true, false)
    }

    /// The `BaseQuantity` attribute: the physical-quantity reference, when
    /// authored.
    pub fn base_quantity(&self) -> ResourceResult<Option<EntityId>> {
        self.record
            .optional_ref("BaseQuantity", "IfcPhysicalQuantity")
    }
}

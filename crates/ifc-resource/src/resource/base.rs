//! Construction-resource occurrence projections.
//!
//! IFC4 and IFC4X3 share one layout. IFC2X3 TC1 declares a reduced one
//! (`ResourceIdentifier`, `ResourceGroup`, `ResourceConsumption`,
//! `BaseQuantity : IfcMeasureWithUnit` on `IfcConstructionResource`, plus
//! `SkillSet`, `Suppliers`/`UsageRatio` and `SubContractor`/`JobDescription`
//! on three subtypes); the IFC2X3-only accessors below read it, and refuse
//! with [`ResourceError::NotInSchema`] under IFC4 and IFC4X3.

use ifc_model::EntityId;

use crate::error::{ResourceError, ResourceResult};
use crate::usage::ResourceTime;
use crate::view::Record;

/// Concrete `IfcConstructionResource` specialization, one of the six
/// resource-occurrence entities this crate projects (all six exist in
/// IFC2X3, IFC4 and IFC4X3).
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
    ///
    /// Under IFC2X3 this reads `ResourceIdentifier`, the `IfcIdentifier`
    /// IFC2X3 declares in the same position.
    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        if self.record.is_ifc2x3() {
            self.record.optional_text("ResourceIdentifier")
        } else {
            self.record.optional_text("Identification")
        }
    }

    /// The `LongDescription` attribute, when authored; IFC2X3 declares none
    /// ([`ResourceError::NotInSchema`]).
    pub fn long_description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LongDescription")
    }

    /// The `PredefinedType` enumeration, when authored; no IFC2X3 resource
    /// declares one ([`ResourceError::NotInSchema`]).
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

    /// The `Usage` attribute: authored `IfcResourceTime`, when present;
    /// IFC2X3 declares neither ([`ResourceError::NotInSchema`]).
    pub fn usage(&self) -> ResourceResult<Option<ResourceTime<'m, 's>>> {
        self.record
            .optional_ref("Usage", "IfcResourceTime")?
            .map(|id| Record::new(self.record.model, self.record.schema, id, "IfcResourceTime"))
            .transpose()
            .map(|record| record.map(ResourceTime::from_record))
    }

    /// The `BaseCosts` attribute: applied-value references, when authored;
    /// IFC2X3 declares none ([`ResourceError::NotInSchema`]).
    pub fn base_costs(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("BaseCosts", "IfcAppliedValue", 1, true, false)
    }

    /// The `BaseQuantity` attribute: the physical-quantity reference, when
    /// authored.
    ///
    /// IFC2X3 declares `BaseQuantity` as an `IfcMeasureWithUnit`, not a
    /// physical quantity, so this refuses with
    /// [`ResourceError::NotInSchema`] there; read it with
    /// [`ConstructionResource::base_quantity_measure`].
    pub fn base_quantity(&self) -> ResourceResult<Option<EntityId>> {
        if self.record.is_ifc2x3() {
            return Err(self.record.not_in_schema("BaseQuantity"));
        }
        self.record
            .optional_ref("BaseQuantity", "IfcPhysicalQuantity")
    }

    /// IFC2X3 `IfcConstructionResource.BaseQuantity`: the
    /// `IfcMeasureWithUnit` reference, when authored. Project it with
    /// [`ResourceView::measure_with_unit`](crate::ResourceView::measure_with_unit).
    ///
    /// IFC4 and IFC4X3 declare `BaseQuantity` as an `IfcPhysicalQuantity`,
    /// read by [`ConstructionResource::base_quantity`]; this refuses there
    /// with [`ResourceError::NotInSchema`].
    pub fn base_quantity_measure(&self) -> ResourceResult<Option<EntityId>> {
        if !self.record.is_ifc2x3() {
            return Err(self.record.not_in_schema("BaseQuantity"));
        }
        self.record
            .optional_ref("BaseQuantity", "IfcMeasureWithUnit")
    }

    /// IFC2X3 `IfcConstructionResource.ResourceGroup` (`IfcLabel`), when
    /// authored. [`ResourceError::NotInSchema`] under IFC4 and IFC4X3.
    pub fn resource_group(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ResourceGroup")
    }

    /// IFC2X3 `IfcConstructionResource.ResourceConsumption`
    /// (`IfcResourceConsumptionEnum`), when authored, checked against the
    /// release's declared members. [`ResourceError::NotInSchema`] under
    /// IFC4 and IFC4X3.
    pub fn resource_consumption(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_enum("ResourceConsumption")
    }

    /// IFC2X3 `IfcLaborResource.SkillSet` (`IfcText`), when authored.
    /// [`ResourceError::NotInSchema`] on any other kind and under IFC4 and
    /// IFC4X3.
    pub fn skill_set(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("SkillSet")
    }

    /// IFC2X3 `IfcConstructionMaterialResource.Suppliers`
    /// (`SET [1:?] OF IfcActorSelect`): person, organization or
    /// person-and-organization references, empty when unset.
    /// [`ResourceError::NotInSchema`] on any other kind and under IFC4 and
    /// IFC4X3.
    pub fn suppliers(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs_select("Suppliers", "IfcActorSelect", ACTOR_SELECT, 1, true, true)
    }

    /// IFC2X3 `IfcConstructionMaterialResource.UsageRatio`
    /// (`IfcRatioMeasure`), when authored; a non-finite value is refused.
    /// [`ResourceError::NotInSchema`] on any other kind and under IFC4 and
    /// IFC4X3.
    pub fn usage_ratio(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_finite_number("UsageRatio")
    }

    /// IFC2X3 `IfcSubContractResource.SubContractor` (`IfcActorSelect`),
    /// when authored. [`ResourceError::NotInSchema`] on any other kind and
    /// under IFC4 and IFC4X3.
    pub fn sub_contractor(&self) -> ResourceResult<Option<EntityId>> {
        self.record
            .optional_ref_select("SubContractor", "IfcActorSelect", ACTOR_SELECT)
    }

    /// IFC2X3 `IfcSubContractResource.JobDescription` (`IfcText`), when
    /// authored. [`ResourceError::NotInSchema`] on any other kind and under
    /// IFC4 and IFC4X3.
    pub fn job_description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("JobDescription")
    }
}

/// `IfcActorSelect` members, identical in IFC2X3, IFC4 and IFC4X3.
const ACTOR_SELECT: &[&str] = &["IfcPerson", "IfcOrganization", "IfcPersonAndOrganization"];

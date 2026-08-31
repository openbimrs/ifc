//! `IfcStructuralAnalysisModel` projection.

use ifc_model::EntityId;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalysisModelType {
    InPlaneLoading2d,
    OutPlaneLoading2d,
    Loading3d,
    UserDefined,
    NotDefined,
}

impl AnalysisModelType {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "IN_PLANE_LOADING_2D" => Some(Self::InPlaneLoading2d),
            "OUT_PLANE_LOADING_2D" => Some(Self::OutPlaneLoading2d),
            "LOADING_3D" => Some(Self::Loading3d),
            "USERDEFINED" => Some(Self::UserDefined),
            "NOTDEFINED" => Some(Self::NotDefined),
            _ => None,
        }
    }

    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::InPlaneLoading2d => "IN_PLANE_LOADING_2D",
            Self::OutPlaneLoading2d => "OUT_PLANE_LOADING_2D",
            Self::Loading3d => "LOADING_3D",
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisModel<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> AnalysisModel<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn object_type(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("ObjectType")
    }

    pub fn predefined_type(&self) -> StructuralResult<AnalysisModelType> {
        let value = self.record.required_enum("PredefinedType")?;
        let parsed = AnalysisModelType::parse(value).ok_or(StructuralError::InvalidValue {
            entity: self.record.id,
            attribute: "PredefinedType",
            expected: "IfcAnalysisModelTypeEnum",
        })?;
        if parsed == AnalysisModelType::UserDefined
            && self
                .object_type()?
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.record.id),
                rule: "USERDEFINED analysis model requires ObjectType",
            });
        }
        Ok(parsed)
    }

    pub fn orientation_of_2d_plane(&self) -> StructuralResult<Option<EntityId>> {
        self.record
            .optional_ref("OrientationOf2DPlane", "IfcAxis2Placement3D")
    }

    pub fn loaded_by(&self) -> StructuralResult<Vec<EntityId>> {
        self.record
            .optional_set_refs("LoadedBy", "IfcStructuralLoadGroup", 1)
    }

    pub fn result_groups(&self) -> StructuralResult<Vec<EntityId>> {
        self.record
            .optional_set_refs("HasResults", "IfcStructuralResultGroup", 1)
    }

    pub fn shared_placement(&self) -> StructuralResult<Option<EntityId>> {
        if !self.record.has_attribute("SharedPlacement") {
            return Ok(None);
        }
        self.record
            .optional_ref("SharedPlacement", "IfcObjectPlacement")
    }
}

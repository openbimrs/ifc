//! `IfcSurfaceStyleLighting` projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcSurfaceStyleLighting`: the four colour
/// coefficients driving IFC's simplified BRDF-like lighting model.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleLighting<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleLighting<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `DiffuseTransmissionColour` attribute. Mandatory.
    pub fn diffuse_transmission_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("DiffuseTransmissionColour", "IfcColourRgb")
    }

    /// The `DiffuseReflectionColour` attribute. Mandatory.
    pub fn diffuse_reflection_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("DiffuseReflectionColour", "IfcColourRgb")
    }

    /// The `TransmissionColour` attribute. Mandatory.
    pub fn transmission_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("TransmissionColour", "IfcColourRgb")
    }

    /// The `ReflectanceColour` attribute. Mandatory.
    pub fn reflectance_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("ReflectanceColour", "IfcColourRgb")
    }
}

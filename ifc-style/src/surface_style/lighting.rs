//! `IfcSurfaceStyleLighting` projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleLighting<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleLighting<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn diffuse_transmission_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("DiffuseTransmissionColour", "IfcColourRgb")
    }

    pub fn diffuse_reflection_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("DiffuseReflectionColour", "IfcColourRgb")
    }

    pub fn transmission_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("TransmissionColour", "IfcColourRgb")
    }

    pub fn reflectance_colour(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("ReflectanceColour", "IfcColourRgb")
    }
}

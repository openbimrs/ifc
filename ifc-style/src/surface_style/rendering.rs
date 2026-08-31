//! `IfcSurfaceStyleRendering` projection.

use ifc_model::{EntityId, Value};

use crate::colour::{optional_colour_or_factor, ColourOrFactor};
use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleRendering<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleRendering<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn surface_colour(&self) -> StyleResult<EntityId> {
        self.record.required_ref("SurfaceColour", "IfcColourRgb")
    }

    pub fn transparency(&self) -> StyleResult<Option<f64>> {
        self.record.optional_normalized("Transparency")
    }

    pub fn diffuse_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "DiffuseColour")
    }

    pub fn transmission_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "TransmissionColour")
    }

    pub fn diffuse_transmission_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "DiffuseTransmissionColour")
    }

    pub fn reflection_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "ReflectionColour")
    }

    pub fn specular_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "SpecularColour")
    }

    pub fn specular_highlight(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("SpecularHighlight")
    }

    pub fn reflectance_method(&self) -> StyleResult<&'m str> {
        self.record.required_enum("ReflectanceMethod")
    }
}

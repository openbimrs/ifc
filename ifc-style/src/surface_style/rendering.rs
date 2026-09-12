//! `IfcSurfaceStyleRendering` projection.

use ifc_model::{EntityId, Value};

use crate::colour::{optional_colour_or_factor, ColourOrFactor};
use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcSurfaceStyleRendering` (a subtype of
/// `IfcSurfaceStyleShading` adding rendering-oriented colour selects and the
/// reflectance method).
#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleRendering<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleRendering<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcSurfaceStyleRendering`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `SurfaceColour` attribute, inherited from `IfcSurfaceStyleShading`. Mandatory.
    pub fn surface_colour(&self) -> StyleResult<EntityId> {
        self.record.required_ref("SurfaceColour", "IfcColourRgb")
    }

    /// The `Transparency` attribute, when authored.
    pub fn transparency(&self) -> StyleResult<Option<f64>> {
        self.record.optional_normalized("Transparency")
    }

    /// The `DiffuseColour` attribute (`IfcColourOrFactor`), when authored.
    pub fn diffuse_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "DiffuseColour")
    }

    /// The `TransmissionColour` attribute (`IfcColourOrFactor`), when authored.
    pub fn transmission_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "TransmissionColour")
    }

    /// The `DiffuseTransmissionColour` attribute (`IfcColourOrFactor`), when authored.
    pub fn diffuse_transmission_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "DiffuseTransmissionColour")
    }

    /// The `ReflectionColour` attribute (`IfcColourOrFactor`), when authored.
    pub fn reflection_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "ReflectionColour")
    }

    /// The `SpecularColour` attribute (`IfcColourOrFactor`), when authored.
    pub fn specular_colour(&self) -> StyleResult<Option<ColourOrFactor>> {
        optional_colour_or_factor(&self.record, "SpecularColour")
    }

    /// The `SpecularHighlight` attribute (`IfcSpecularHighlightSelect`:
    /// specular exponent or roughness), when authored. Returned raw because
    /// the select is not narrowed to one representation here.
    pub fn specular_highlight(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("SpecularHighlight")
    }

    /// The `ReflectanceMethod` attribute: the `IfcReflectanceMethodEnum`
    /// token (e.g. `"PHYSICAL"`, `"BLINN"`, `"FLAT"`) selecting the shading
    /// model this rendering data feeds. Mandatory.
    pub fn reflectance_method(&self) -> StyleResult<&'m str> {
        self.record.required_enum("ReflectanceMethod")
    }
}

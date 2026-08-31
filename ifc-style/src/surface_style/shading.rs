//! Surface colour and transparency projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleShading<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleShading<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn surface_colour(&self) -> StyleResult<EntityId> {
        self.record.required_ref("SurfaceColour", "IfcColourRgb")
    }

    /// IFC2x3 shading has no transparency slot; that is reported as absent.
    pub fn transparency(&self) -> StyleResult<Option<f64>> {
        self.record.optional_normalized("Transparency")
    }
}

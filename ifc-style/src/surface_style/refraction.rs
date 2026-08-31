//! `IfcSurfaceStyleRefraction` projection.

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleRefraction<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleRefraction<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn refraction_index(&self) -> StyleResult<Option<f64>> {
        self.record.optional_number("RefractionIndex")
    }

    pub fn dispersion_factor(&self) -> StyleResult<Option<f64>> {
        self.record.optional_number("DispersionFactor")
    }
}

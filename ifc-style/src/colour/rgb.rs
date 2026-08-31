//! Strict `IfcColourRgb` projection.

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct ColourRgb<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ColourRgb<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn red(&self) -> StyleResult<f64> {
        self.record.normalized("Red")
    }

    pub fn green(&self) -> StyleResult<f64> {
        self.record.normalized("Green")
    }

    pub fn blue(&self) -> StyleResult<f64> {
        self.record.normalized("Blue")
    }

    pub fn channels(&self) -> StyleResult<[f64; 3]> {
        Ok([self.red()?, self.green()?, self.blue()?])
    }
}

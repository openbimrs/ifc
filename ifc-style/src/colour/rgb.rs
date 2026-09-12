//! Strict `IfcColourRgb` projection.

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcColourRgb`: a normalized red/green/blue colour.
#[derive(Debug, Clone, Copy)]
pub struct ColourRgb<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ColourRgb<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `Red` component, normalized to `[0, 1]`.
    pub fn red(&self) -> StyleResult<f64> {
        self.record.normalized("Red")
    }

    /// The `Green` component, normalized to `[0, 1]`.
    pub fn green(&self) -> StyleResult<f64> {
        self.record.normalized("Green")
    }

    /// The `Blue` component, normalized to `[0, 1]`.
    pub fn blue(&self) -> StyleResult<f64> {
        self.record.normalized("Blue")
    }

    /// The `[Red, Green, Blue]` components as one array.
    pub fn channels(&self) -> StyleResult<[f64; 3]> {
        Ok([self.red()?, self.green()?, self.blue()?])
    }
}

//! Project unit factors a caller resolved for hatch measures.
//!
//! `ifc-style` links no unit resolver (ADR 0003): the caller reads the
//! project's `IfcUnitAssignment` and passes the two factors this crate needs.
//! A factor the project does not state stays `None`, and a measure that needs
//! it is refused, never read as metres or radians.

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};

/// Project unit factors a caller resolved (ifc-style links no unit resolver; ADR 0003).
///
/// The fields are private so that every value is finite and positive: build
/// one with [`StyleUnits::new`] or take [`StyleUnits::SI`].
///
/// ```compile_fail
/// use ifc_style::StyleUnits;
/// let _ = StyleUnits { metres_per_length: Some(f64::NAN), radians_per_angle: None };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleUnits {
    metres_per_length: Option<f64>,
    radians_per_angle: Option<f64>,
}

impl StyleUnits {
    /// Metres and radians; for files known to be SI, never a fallback for undeclared units.
    pub const SI: Self = Self {
        metres_per_length: Some(1.0),
        radians_per_angle: Some(1.0),
    };

    /// Metres per project length unit; `None` when the project declares no `LENGTHUNIT`.
    #[must_use]
    pub fn metres_per_length(&self) -> Option<f64> {
        self.metres_per_length
    }

    /// Radians per project plane angle unit; `None` when no `PLANEANGLEUNIT`.
    #[must_use]
    pub fn radians_per_angle(&self) -> Option<f64> {
        self.radians_per_angle
    }

    /// Unit factors from the two project factors; `None` states "not declared".
    ///
    /// # Errors
    ///
    /// [`StyleError::InvalidUnits`] for a factor that is not finite and
    /// greater than zero.
    pub fn new(
        metres_per_length: Option<f64>,
        radians_per_angle: Option<f64>,
    ) -> StyleResult<Self> {
        check("metres_per_length", metres_per_length)?;
        check("radians_per_angle", radians_per_angle)?;
        Ok(Self {
            metres_per_length,
            radians_per_angle,
        })
    }

    // Read by the fill resolver; the allow goes with it.
    #[allow(dead_code)]
    /// `value` in metres, read by `entity`.
    pub(crate) fn length(&self, entity: EntityId, value: f64) -> StyleResult<f64> {
        scale(self.metres_per_length, entity, "LENGTHUNIT", value)
    }

    #[allow(dead_code)]
    /// `value` in radians, read by `entity`.
    pub(crate) fn angle(&self, entity: EntityId, value: f64) -> StyleResult<f64> {
        scale(self.radians_per_angle, entity, "PLANEANGLEUNIT", value)
    }
}

fn check(factor: &'static str, value: Option<f64>) -> StyleResult<()> {
    match value {
        Some(value) if !(value.is_finite() && value > 0.0) => {
            Err(StyleError::InvalidUnits { factor, value })
        }
        _ => Ok(()),
    }
}

#[allow(dead_code)]
fn scale(
    factor: Option<f64>,
    entity: EntityId,
    unit: &'static str,
    value: f64,
) -> StyleResult<f64> {
    factor
        .map(|factor| value * factor)
        .ok_or(StyleError::UnitUndeclared { entity, unit })
}

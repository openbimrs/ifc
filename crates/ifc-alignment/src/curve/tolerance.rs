//! How far apart two authored lengths may be and still name the same seam.
//!
//! Every vertical segment restates where it starts (`StartDistAlong`,
//! `StartHeight`), and those restatements are compared with where the
//! previous segment ends. The comparison needs a tolerance, and the honest
//! source for one is the file itself:
//! `IfcGeometricRepresentationContext.Precision` is, in the schema's own
//! words, "the tolerance under which two given points are still assumed to
//! be identical" (IFC4.3 ADD2, `IfcGeometricRepresentationContext`).
//!
//! Exporters that round stations and heights to their working precision
//! state that precision there. buildingSMART's own alignment reference
//! datasets (IFC4.x-IF `BC003_*`) declare `Precision = 1E-4` and carry seam
//! residuals up to 1E-5 m in `StartDistAlong` and 8.5E-6 m in
//! `StartHeight`; a fixed 1E-9 relative test refuses every one of them.
//!
//! The rule here:
//!
//! - a length seam (`StartDistAlong`, `StartHeight`) is accepted when the
//!   two values differ by at most the larger of `1e-9 * max(|a|, |b|, 1)`
//!   (floating-point rounding of the seam arithmetic itself) and the
//!   declared precision converted to metres;
//! - the declared precision is read from the model's 3D
//!   `IfcGeometricRepresentationContext`s (subcontexts derive theirs); when
//!   several declare one, the coarsest applies, because the profile is not
//!   tied to any one representation;
//! - a declared precision coarser than [`SeamTolerance::MAX_PRECISION`]
//!   (1 mm) is capped there, so a file can never talk the check into joining
//!   a visible step;
//! - with no declared precision the rounding term alone applies, which is
//!   the behaviour before this tolerance existed;
//! - gradients are ratios, not lengths, so no length unit or length
//!   precision applies to them; they keep the `1e-9` absolute test.

use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

/// The tolerance a vertical profile's seams are checked against.
///
/// Build it from the model with [`SeamTolerance::for_model`], from a known
/// precision with [`SeamTolerance::from_precision`], or use
/// [`SeamTolerance::strict`] for the floating-point rounding term alone.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SeamTolerance {
    /// Absolute length tolerance in metres, already capped.
    length: f64,
}

impl Default for SeamTolerance {
    fn default() -> Self {
        Self::strict()
    }
}

impl SeamTolerance {
    /// The coarsest precision honoured, in metres.
    ///
    /// A file declaring a coarser `Precision` is checked at 1 mm instead: a
    /// step a surveyor could see is never joined on the file's say-so.
    pub const MAX_PRECISION: f64 = 1e-3;

    /// Only the floating-point rounding term: `1e-9 * max(|a|, |b|, 1)`.
    #[must_use]
    pub fn strict() -> Self {
        Self { length: 0.0 }
    }

    /// Accept length seams within `precision_metres`, capped at
    /// [`Self::MAX_PRECISION`].
    ///
    /// # Errors
    ///
    /// Refuses a negative or non-finite precision with
    /// [`AlignmentError::InvalidUnits`].
    pub fn from_precision(precision_metres: f64) -> AlignmentResult<Self> {
        if !precision_metres.is_finite() || precision_metres < 0.0 {
            return Err(AlignmentError::InvalidUnits {
                detail: "seam precision must be finite and non-negative",
            });
        }
        Ok(Self {
            length: precision_metres.min(Self::MAX_PRECISION),
        })
    }

    /// The tolerance the model itself declares.
    ///
    /// Reads `Precision` from every `IfcGeometricRepresentationContext`
    /// whose `CoordinateSpaceDimension` is 3 and takes the coarsest,
    /// converted from the project length unit with
    /// `units.length_to_metres`. No declared precision gives
    /// [`Self::strict`].
    ///
    /// # Errors
    ///
    /// Refuses invalid `units`, and a declared `Precision` that is not a
    /// finite, positive number (`InvalidAttribute` naming the context).
    pub fn for_model(model: &Model, units: AlignmentUnits) -> AlignmentResult<Self> {
        if !units.length_to_metres.is_finite() || units.length_to_metres <= 0.0 {
            return Err(AlignmentError::InvalidUnits {
                detail: "length factor must be finite and positive",
            });
        }
        let mut coarsest: Option<f64> = None;
        for (id, entity) in model.iter() {
            // Exact type: a subcontext's Precision is derived (`*`) from its
            // parent, which this loop already visits.
            if !entity
                .type_name
                .eq_ignore_ascii_case("IFCGEOMETRICREPRESENTATIONCONTEXT")
            {
                continue;
            }
            if let Some(precision) = declared_3d_precision(id, &entity.attributes)? {
                let metres = precision * units.length_to_metres;
                coarsest = Some(coarsest.map_or(metres, |c: f64| c.max(metres)));
            }
        }
        coarsest.map_or(Ok(Self::strict()), Self::from_precision)
    }

    /// The absolute length tolerance in metres, after the cap.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.length
    }

    /// Whether two lengths, in metres, name the same seam.
    pub(crate) fn same_length(&self, left: f64, right: f64) -> bool {
        (left - right).abs() <= rounding(left, right).max(self.length)
    }

    /// Whether two gradients name the same seam: rounding only, no length
    /// precision, because a gradient is a ratio.
    pub(crate) fn same_gradient(&self, left: f64, right: f64) -> bool {
        (left - right).abs() <= rounding(left, right)
    }
}

/// Floating-point rounding of the seam arithmetic: `1e-9` relative, floor 1.
fn rounding(left: f64, right: f64) -> f64 {
    1e-9 * left.abs().max(right.abs()).max(1.0)
}

/// `Precision` of a 3D context, if declared.
///
/// IFC4X3_ADD2 `IfcGeometricRepresentationContext`: `ContextIdentifier`,
/// `ContextType` (inherited), then `CoordinateSpaceDimension` at slot 2 and
/// `Precision` at slot 3.
fn declared_3d_precision(
    id: EntityId,
    attributes: &[ifc_model::Value],
) -> AlignmentResult<Option<f64>> {
    const DIMENSION: usize = 2;
    const PRECISION: usize = 3;
    let dimension = attributes
        .get(DIMENSION)
        .and_then(|value| value.unwrap_typed().as_f64());
    if dimension != Some(3.0) {
        return Ok(None);
    }
    match attributes.get(PRECISION) {
        None | Some(ifc_model::Value::Null | ifc_model::Value::Derived) => Ok(None),
        Some(value) => match value.unwrap_typed().as_f64() {
            Some(precision) if precision.is_finite() && precision > 0.0 => Ok(Some(precision)),
            _ => Err(AlignmentError::InvalidAttribute {
                entity: id,
                index: PRECISION,
                name: "Precision",
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::{Entity, Value};
    use std::sync::Arc;

    fn metres() -> AlignmentUnits {
        AlignmentUnits {
            length_to_metres: 1.0,
            angle_to_radians: 1.0,
        }
    }

    fn context(model: &mut Model, id: u64, dimension: i64, precision: Value) {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCGEOMETRICREPRESENTATIONCONTEXT",
                vec![
                    Value::Null,
                    Value::Text(Arc::from("Model")),
                    Value::Integer(dimension),
                    precision,
                    Value::Null,
                    Value::Null,
                ],
            ),
        );
    }

    #[test]
    fn no_declared_precision_is_strict() {
        let mut model = Model::new();
        context(&mut model, 1, 3, Value::Null);
        assert_eq!(
            SeamTolerance::for_model(&model, metres()),
            Ok(SeamTolerance::strict())
        );
    }

    #[test]
    fn the_coarsest_3d_precision_applies_and_2d_contexts_are_ignored() {
        let mut model = Model::new();
        context(&mut model, 1, 3, Value::Real(1e-6));
        context(&mut model, 2, 3, Value::Real(1e-4));
        context(&mut model, 3, 2, Value::Real(1e-2));
        let tolerance = SeamTolerance::for_model(&model, metres()).expect("tolerance");
        assert_eq!(tolerance.length(), 1e-4);
    }

    #[test]
    fn precision_is_converted_from_the_project_length_unit() {
        let mut model = Model::new();
        // 0.1 mm declared in millimetres.
        context(&mut model, 1, 3, Value::Real(0.1));
        let millimetres = AlignmentUnits {
            length_to_metres: 0.001,
            angle_to_radians: 1.0,
        };
        let tolerance = SeamTolerance::for_model(&model, millimetres).expect("tolerance");
        assert!((tolerance.length() - 1e-4).abs() < 1e-18);
    }

    #[test]
    fn a_coarse_precision_is_capped_at_one_millimetre() {
        let mut model = Model::new();
        context(&mut model, 1, 3, Value::Real(0.5));
        let tolerance = SeamTolerance::for_model(&model, metres()).expect("tolerance");
        assert_eq!(tolerance.length(), SeamTolerance::MAX_PRECISION);
        assert!(!tolerance.same_length(52.0, 52.002));
    }

    #[test]
    fn a_non_positive_precision_is_refused_by_name() {
        let mut model = Model::new();
        context(&mut model, 7, 3, Value::Real(-1e-5));
        assert_eq!(
            SeamTolerance::for_model(&model, metres()),
            Err(AlignmentError::InvalidAttribute {
                entity: EntityId(7),
                index: 3,
                name: "Precision",
            })
        );
    }

    #[test]
    fn gradients_ignore_the_length_precision() {
        let tolerance = SeamTolerance::from_precision(1e-4).expect("tolerance");
        assert!(tolerance.same_length(100.0, 100.00009));
        assert!(!tolerance.same_gradient(0.02, 0.02009));
    }
}

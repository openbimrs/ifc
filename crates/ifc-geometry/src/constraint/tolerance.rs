//! The model's own tolerance, for deciding whether two frames agree.
//!
//! IFC defines `IfcGeometricRepresentationContext.Precision` as "the
//! tolerance under which two given points are still assumed to be
//! identical". Placement resolution compares frames in two places -- a
//! cached `CartesianPosition` against the derived one (#354), and a stated
//! `PlacementRelTo` against the frame IFC says it must name (#357, #362) --
//! and both use that tolerance rather than one of our own.

use crate::error::{GeometryError, GeometryResult};
use crate::input::context::all_contexts;
use crate::transform::Transform;

/// IFC4 ADD2 TC1 `IfcGeometricRepresentationSubContext.Precision`:
/// `NVL(ParentContext.Precision, 1.E-5)`.
const IFC_DEFAULT_PRECISION: f64 = 1e-5;

/// How far two unit axis vectors may differ and still be the same axis.
///
/// Direction ratios are dimensionless, so `Precision` (a length) does not
/// apply to them; this is floating-point rounding with headroom.
const AXIS_TOLERANCE: f64 = 1e-9;

/// The coarsest `Precision` any 3D root context declares, in FILE units.
///
/// Sub-contexts derive theirs from it. A model that declares none gets
/// IFC's default, `1.E-5` project units.
///
/// # Errors
///
/// A declared `Precision` that is not a finite positive number is
/// [`GeometryError::Degenerate`] naming the context: a check against it
/// would accept or refuse everything.
pub(crate) fn model_precision(model: &ifc_model::Model) -> GeometryResult<f64> {
    let mut coarsest: Option<f64> = None;
    for context in all_contexts(model) {
        if context.is_sub_context() || context.coordinate_space_dimension(model) != Some(3) {
            continue;
        }
        let Some(precision) = context.precision(model) else {
            continue;
        };
        if !(precision.is_finite() && precision > 0.0) {
            return Err(GeometryError::Degenerate {
                entity: context.id(),
                type_name: "IFCGEOMETRICREPRESENTATIONCONTEXT".into(),
                detail: format!(
                    "Precision {precision} is not a finite positive tolerance, so placements \
                     cannot be compared against it"
                ),
            });
        }
        coarsest = Some(coarsest.map_or(precision, |c: f64| c.max(precision)));
    }
    Ok(coarsest.unwrap_or(IFC_DEFAULT_PRECISION))
}

/// The tolerance two points at `a` and `b` are compared at: `precision`,
/// floored at floating-point rounding (1e-9 relative to their magnitude,
/// as the alignment seams use).
pub(crate) fn point_tolerance(precision: f64, a: [f64; 3], b: [f64; 3]) -> f64 {
    let magnitude = a.iter().chain(b.iter()).fold(1.0f64, |m, v| m.max(v.abs()));
    precision.max(1e-9 * magnitude)
}

/// Distance between two points.
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Do two frames, in FILE units, describe the same coordinate system?
///
/// Origins within the model's `Precision`, axes within rounding.
pub(crate) fn frames_agree(
    model: &ifc_model::Model,
    a: &Transform,
    b: &Transform,
) -> GeometryResult<bool> {
    let tolerance = point_tolerance(model_precision(model)?, a.origin, b.origin);
    let origin = distance(a.origin, b.origin);
    let axes = a
        .basis
        .iter()
        .flatten()
        .zip(b.basis.iter().flatten())
        .all(|(x, y)| (x - y).abs() <= AXIS_TOLERANCE);
    Ok(origin.is_finite() && origin <= tolerance && axes)
}

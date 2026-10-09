//! The model's own tolerance, for deciding whether two frames agree.
//!
//! IFC defines `IfcGeometricRepresentationContext.Precision` as "the
//! tolerance under which two given points are still assumed to be
//! identical". Placement resolution compares frames in two places -- a
//! cached `CartesianPosition` against the derived one (#354), and a stated
//! `PlacementRelTo` against the frame IFC says it must name (#357, #362) --
//! and both use that tolerance rather than one of our own. Boundary joints
//! and the three points of an `IfcArcIndex` ([`arc_points`]) are judged
//! against it too.

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

/// [`model_precision`] in metres: the tolerance under which two points of
/// this model are "still assumed to be identical".
pub(crate) fn model_precision_metres(
    model: &ifc_model::Model,
    units: &crate::units::UnitScale,
) -> GeometryResult<f64> {
    Ok(units.length(model_precision(model)?))
}

/// Do two 2D points, in metres, coincide within `precision`?
///
/// The tolerance is [`point_tolerance`]'s, so it never falls below
/// floating-point rounding. A non-finite distance never coincides.
pub(crate) fn points_coincide(precision: f64, a: [f64; 2], b: [f64; 2]) -> bool {
    let (a, b) = ([a[0], a[1], 0.0], [b[0], b[1], 0.0]);
    let gap = distance(a, b);
    gap.is_finite() && gap <= point_tolerance(precision, a, b)
}

/// How the three points of an `IfcArcIndex` read, within the model's
/// `Precision` (#335, #396).
///
/// IFC4 ADD2 TC1 and IFC4X3 ADD2 state `IfcIndexedPolyCurve`'s informal
/// proposition identically: "The three points shall not be co-linear. In
/// case that this informal proposition is not maintained, the arc segment
/// shall be treated as a polyline segment." Coincidence and collinearity are
/// judged "after taking the Precision factor into account, given by the
/// applicable IfcGeometricRepresentationContext". Every reader of an
/// `IfcArcIndex` classifies it here -- the profile and half-space boundary
/// reader and the general curve lowering -- so the same three points never
/// read as an arc in one and a polyline in the other.
#[cfg(feature = "lowering")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArcPoints {
    /// Two of the points coincide: no circle and no polyline segment is
    /// defined, and the schema states no fallback.
    Coincident,
    /// A coordinate, or arithmetic on the coordinates, is not finite.
    NonFinite,
    /// Three distinct collinear points: the polyline segment
    /// start -> mid -> end. When the middle point lies between the others
    /// that path is the one straight edge start -> end
    /// (`through_mid == false`); otherwise it runs out to the middle point
    /// and back, two edges (`through_mid == true`).
    Collinear {
        /// Whether the polyline needs the middle point as a vertex.
        through_mid: bool,
    },
    /// Three distinct points off one line: a circular arc.
    Circular,
}

/// Classify the three points of an `IfcArcIndex`, in metres; see
/// [`ArcPoints`]. 2D points are passed with `z = 0`.
///
/// Coincidence compares each pair at [`point_tolerance`]. Collinearity
/// compares the middle point's distance from the chord start -> end (the
/// arc's sagitta, seen from that point) at the same tolerance, so a middle
/// point within `Precision` of the line through the others is on it.
#[cfg(feature = "lowering")]
pub(crate) fn arc_points(
    precision: f64,
    start: [f64; 3],
    mid: [f64; 3],
    end: [f64; 3],
) -> ArcPoints {
    if !start.iter().chain(&mid).chain(&end).all(|v| v.is_finite()) {
        return ArcPoints::NonFinite;
    }
    let coincide = |a: [f64; 3], b: [f64; 3]| {
        let gap = distance(a, b);
        gap.is_finite() && gap <= point_tolerance(precision, a, b)
    };
    if coincide(start, mid) || coincide(mid, end) || coincide(start, end) {
        return ArcPoints::Coincident;
    }
    let u = [mid[0] - start[0], mid[1] - start[1], mid[2] - start[2]];
    let chord = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let chord_sq = chord[0] * chord[0] + chord[1] * chord[1] + chord[2] * chord[2];
    let normal = [
        u[1] * chord[2] - u[2] * chord[1],
        u[2] * chord[0] - u[0] * chord[2],
        u[0] * chord[1] - u[1] * chord[0],
    ];
    let sagitta = distance(normal, [0.0; 3]) / chord_sq.sqrt();
    if sagitta.is_nan() {
        return ArcPoints::NonFinite;
    }
    let tolerance =
        point_tolerance(precision, start, end).max(point_tolerance(precision, mid, mid));
    if sagitta > tolerance {
        return ArcPoints::Circular;
    }
    let along = (u[0] * chord[0] + u[1] * chord[1] + u[2] * chord[2]) / chord_sq;
    ArcPoints::Collinear {
        through_mid: !(0.0..=1.0).contains(&along),
    }
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

#[cfg(all(test, feature = "lowering"))]
mod tests {
    use super::{arc_points, ArcPoints};

    const PRECISION: f64 = 1e-5;

    #[test]
    fn three_points_off_a_line_are_an_arc() {
        let arc = arc_points(PRECISION, [0.0; 3], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]);
        assert_eq!(arc, ArcPoints::Circular);
    }

    #[test]
    fn collinear_points_are_a_polyline_through_the_middle_only_when_needed() {
        let between = arc_points(PRECISION, [0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        assert_eq!(between, ArcPoints::Collinear { through_mid: false });
        let beyond = arc_points(PRECISION, [0.0; 3], [3.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        assert_eq!(beyond, ArcPoints::Collinear { through_mid: true });
        let before = arc_points(PRECISION, [0.0; 3], [-1.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        assert_eq!(before, ArcPoints::Collinear { through_mid: true });
    }

    /// The sagitta is compared with `Precision`, not an absolute bound on a
    /// squared area: 4e-6 m off a 2 m chord is on it under 1e-5 m and off it
    /// under 1e-6 m, in 3D as in 2D.
    #[test]
    fn collinearity_is_judged_within_precision() {
        let (start, mid, end) = ([0.0, 0.0, 5.0], [1.0, 0.0, 5.000004], [2.0, 0.0, 5.0]);
        assert_eq!(
            arc_points(1e-5, start, mid, end),
            ArcPoints::Collinear { through_mid: false }
        );
        assert_eq!(arc_points(1e-6, start, mid, end), ArcPoints::Circular);
    }

    #[test]
    fn coincident_points_are_neither_arc_nor_polyline() {
        let near = [0.000004, 0.0, 0.0];
        assert_eq!(
            arc_points(PRECISION, [0.0; 3], near, [2.0, 0.0, 0.0]),
            ArcPoints::Coincident
        );
        assert_eq!(
            arc_points(PRECISION, [0.0; 3], [1.0, 1.0, 0.0], near),
            ArcPoints::Coincident
        );
        assert_ne!(
            arc_points(1e-6, [0.0; 3], [1.0, 1.0, 0.0], near),
            ArcPoints::Coincident
        );
    }

    #[test]
    fn non_finite_points_or_arithmetic_are_reported() {
        let nan = [f64::NAN, 0.0, 0.0];
        assert_eq!(
            arc_points(PRECISION, [0.0; 3], nan, [1.0, 0.0, 0.0]),
            ArcPoints::NonFinite
        );
        let huge = arc_points(
            PRECISION,
            [-1e300, 0.0, 0.0],
            [0.0, 1e300, 0.0],
            [1e300, 0.0, 0.0],
        );
        assert_eq!(huge, ArcPoints::NonFinite);
    }
}

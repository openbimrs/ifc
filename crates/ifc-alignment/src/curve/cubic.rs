//! The `CUBIC` horizontal transition: an exact cubic parabola read by arc
//! length (#90).
//!
//! IFC4.3 ADD2 (`IfcAlignmentHorizontalSegmentTypeEnum`, 8.7.2.2) defines
//! `CUBIC` as "a transition segment where x and y coordinates obey a cubic
//! formula", for alignments `y = x^3 / (6 R L)` in the segment's own frame,
//! and `IfcAlignmentHorizontalSegment.SegmentLength` as the length along the
//! curve. The shape is an exact polynomial; where it ends is not elementary:
//! the abscissa at arc length `L` inverts
//! `s(x) = integral_0^x sqrt(1 + (t^2 / (2 R L))^2) dt`, an elliptic
//! integral. So the lowering stores the shape and the length, and leaves the
//! inversion to an evaluator:
//!
//! - **On its own** (the per-segment curve and the composite): a cubic
//!   Bezier, `Curve2::BSpline`, over `x in [0, L]`, trimmed from parameter
//!   `0` to `TrimSelector::ArcLength(L)`. The parameter is `x` itself, so the
//!   curve's speed is at least 1 and arc length `L` is reached by `x = L` at
//!   the latest: the trim always lies inside the stored span.
//! - **In a plan** (`Elevated3.plan`): a `ChainPiece2::Parametric` of the
//!   same cubic in the segment's local frame, read by arc length over `L`.
//!
//! The control points are the power-to-Bernstein conversion of `(x, a x^3)`
//! over `[0, L]`: `(0, 0)`, `(L/3, 0)`, `(2L/3, 0)`, `(L, a L^3)`. That is a
//! change of basis, exact up to the rounding of `a L^3`, not a fit.
//!
//! # Only a CUBIC that leaves a straight
//!
//! `y = x^3 / (6 R L)` has zero curvature at its start, so it is the CUBIC
//! from `StartRadiusOfCurvature = 0` (straight) to `EndRadiusOfCurvature =
//! R`. IFC4.3 states no formula for a CUBIC that starts curved; one would
//! have to begin part-way along some cubic, at a point the standard does not
//! fix, so it stays a typed refusal rather than a guessed offset.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{BSplineCurve2, Curve2, KnotSpec};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::HorizontalSegment;

/// The source token this module lowers.
pub(super) const CUBIC: &str = "CUBIC";

/// The cubic parabola of a `CUBIC` segment, in `frame`: `frame.origin` at
/// the segment start, `frame.x` along its start tangent, `frame.y` to its
/// left.
///
/// Its parameter is the local abscissa `x`, over `[0, L]`.
///
/// # Errors
///
/// [`refuse`]'s refusals: malformed data first, then a CUBIC that does not
/// leave a straight.
pub(super) fn cubic_curve(segment: &HorizontalSegment, frame: Frame2) -> AlignmentResult<Curve2> {
    if let Some(refusal) = refuse(segment) {
        return Err(refusal);
    }
    let length = segment.segment_length;
    // y = a x^3 with a = 1 / (6 R L); R signed, positive turning left.
    let rise = length * length / (6.0 * segment.end_radius);
    let local = [
        (0.0, 0.0),
        (length / 3.0, 0.0),
        (2.0 * length / 3.0, 0.0),
        (length, rise),
    ];
    let control_points: Vec<Point2> = local
        .iter()
        .map(|(x, y)| frame.origin + frame.x * *x + frame.y * *y)
        .collect();
    if !rise.is_finite()
        || control_points
            .iter()
            .any(|p| !(p.x.is_finite() && p.y.is_finite()))
    {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CUBIC control points must be finite",
        });
    }
    Ok(Curve2::BSpline(BSplineCurve2 {
        degree: 3,
        control_points,
        knots: vec![0.0, length],
        multiplicities: vec![4, 4],
        weights: None,
        closed: false,
        self_intersect: Some(false),
        knot_spec: KnotSpec::PiecewiseBezier,
    }))
}

/// The segment's own start frame in world coordinates.
pub(super) fn start_frame(segment: &HorizontalSegment) -> Frame2 {
    let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
    Frame2 {
        origin: segment.start_point,
        x: direction,
        y: Vec2::new(-direction.y, direction.x),
    }
}

/// The unit frame at the origin: a chain piece's local frame.
pub(super) fn local_frame() -> Frame2 {
    Frame2 {
        origin: Point2::new(0.0, 0.0),
        x: Vec2::new(1.0, 0.0),
        y: Vec2::new(0.0, 1.0),
    }
}

/// Why a `CUBIC` cannot be lowered, or `None` when it can.
///
/// Malformed data is named as invalid before the capability gap, so a
/// caller can tell a file error from a curve this crate does not model.
pub(super) fn refuse(segment: &HorizontalSegment) -> Option<AlignmentError> {
    let invalid = |detail| {
        Some(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail,
        })
    };
    if !(segment.segment_length.is_finite() && segment.segment_length > 0.0) {
        return invalid("CUBIC requires a finite, positive segment length");
    }
    if !(segment.start_radius.is_finite() && segment.end_radius.is_finite()) {
        return invalid("CUBIC endpoint radii must be finite");
    }
    if segment.start_radius == segment.end_radius {
        return invalid(
            "CUBIC is a transition and must change curvature: its start and end radii must differ",
        );
    }
    if segment.start_radius != 0.0 {
        return Some(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: CUBIC.to_owned(),
            detail: "IFC4.3 defines CUBIC as y = x^3/(6RL), which leaves a straight; a CUBIC \
                     starting curved (StartRadiusOfCurvature != 0) would begin part-way along \
                     a cubic at a point the standard does not fix",
        });
    }
    None
}

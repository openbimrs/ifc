//! Seams between consecutive horizontal segments, and the rule for checking
//! them without numerical integration.
//!
//! Every `IfcAlignmentHorizontalSegment` restates its own `StartPoint` and
//! `StartDirection`. IFC names the checks that follow: the computed end of
//! the previous segment should match both. This crate computes nothing it
//! cannot compute in closed form, so the rule is:
//!
//! - **Direction** is checkable after every segment with a curvature law.
//!   Each law this crate lowers has an elementary antiderivative, so the
//!   heading at a segment's end is its `StartDirection` plus a closed-form
//!   turning integral. A `CUBIC` has no curvature law in arc length: its
//!   end heading is `atan(x_e^2 / (2 R L))` at the abscissa `x_e` where its
//!   arc length reaches `L`, an elliptic-integral inverse.
//! - **Position** is checkable only after a `LINE` or `CIRCULARARC`, whose
//!   end point is elementary. After a transition spiral the end point is a
//!   Fresnel-type integral, after a `CUBIC` the same elliptic inverse.
//!   Rather than quadrature it, or refusing every real alignment, the seam
//!   is recorded as [`SeamCheck::Authored`]: the authored `StartPoint` (and,
//!   after a `CUBIC`, the authored `StartDirection`) is named as unverified.
//!
//! A mismatch that IS checkable is refused, never smoothed over.

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{CurvatureLaw, Intrinsic2};
use ifc_model::EntityId;

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::{HorizontalSegment, HorizontalSegmentType};

/// Largest gap, in metres, accepted between a closed-form end point and the
/// next segment's authored `StartPoint`.
pub(super) const POSITION_TOLERANCE: f64 = 1e-6;

/// Largest difference, in radians, accepted between a closed-form end
/// heading and the next segment's authored `StartDirection`.
pub(super) const DIRECTION_TOLERANCE: f64 = 1e-6;

/// How the position at a seam between two horizontal segments was checked.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeamCheck {
    /// The previous segment's end point is closed form and matches the
    /// authored `StartPoint` of the next segment within 1e-6 m.
    Verified,
    /// The previous segment is a transition spiral or a `CUBIC`, whose end
    /// point is a non-elementary integral this crate does not evaluate. The
    /// authored `StartPoint` of the next segment was accepted as stated, not
    /// verified. After a `CUBIC` its `StartDirection` is not verified either:
    /// the cubic's end heading is not closed form.
    Authored,
}

/// One seam between consecutive horizontal segments.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HorizontalSeam {
    /// The segment that ends at this seam.
    pub previous: EntityId,
    /// The segment that starts at this seam.
    pub next: EntityId,
    /// Distance along the layout from its first segment to the seam, in
    /// metres: the sum of the preceding `SegmentLength`s.
    pub distance_along: f64,
    /// How the seam position was checked.
    pub position: SeamCheck,
}

/// Exact end point of a segment, in closed form.
///
/// `None` for every transition-spiral family and `CUBIC`: their end point
/// is a Fresnel-type or elliptic integral, so there is no closed form to
/// return and this crate will not quadrature one into existence.
pub(super) fn closed_form_end_point(segment: &HorizontalSegment) -> Option<Point2> {
    match segment.segment_type {
        HorizontalSegmentType::Line => {
            let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
            Some(segment.start_point + direction * segment.segment_length)
        }
        HorizontalSegmentType::CircularArc if segment.start_radius != 0.0 => {
            let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
            let left = Vec2::new(-direction.y, direction.x);
            let centre = segment.start_point + left * segment.start_radius;
            let sweep = segment.segment_length / segment.start_radius;
            let radial = segment.start_point - centre;
            let (sin_s, cos_s) = sweep.sin_cos();
            Some(
                centre
                    + Vec2::new(
                        radial.x * cos_s - radial.y * sin_s,
                        radial.x * sin_s + radial.y * cos_s,
                    ),
            )
        }
        _ => None,
    }
}

/// Check the position where `next` starts against where `previous` ends.
///
/// Returns how the seam was checked, or refuses a closed-form mismatch.
pub(super) fn check_position(
    previous: &HorizontalSegment,
    next: &HorizontalSegment,
    distance_along: f64,
) -> AlignmentResult<HorizontalSeam> {
    let position = match closed_form_end_point(previous) {
        Some(end) if end.distance(next.start_point) <= POSITION_TOLERANCE => SeamCheck::Verified,
        Some(_) => {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(next.entity),
                rule: "consecutive horizontal segments must share an endpoint exactly",
            })
        }
        None => SeamCheck::Authored,
    };
    Ok(HorizontalSeam {
        previous: previous.entity,
        next: next.entity,
        distance_along,
        position,
    })
}

/// Check the heading where `next` starts against where `previous` ends.
///
/// `law` is the previous segment's exact curvature law. The end heading is
/// its `StartDirection` plus the closed-form turning integral of that law,
/// so this check holds for spirals too. Directions are compared modulo a
/// full turn: IFC allows the same bearing to be written as `-pi/2` or
/// `3pi/2`.
pub(super) fn check_direction(
    previous: &HorizontalSegment,
    law: &CurvatureLaw,
    next: &HorizontalSegment,
) -> AlignmentResult<()> {
    let turning = if previous.segment_length == 0.0 {
        0.0
    } else {
        Intrinsic2::new(unit_frame(), law.clone(), previous.segment_length)
            .total_turning()
            .ok_or(AlignmentError::InvalidSegment {
                entity: previous.entity,
                detail: "horizontal segment curvature law has no finite turning integral",
            })?
    };
    let end = previous.start_direction + turning;
    let difference = (next.start_direction - end).rem_euclid(core::f64::consts::TAU);
    let difference = difference.min(core::f64::consts::TAU - difference);
    if difference.is_finite() && difference <= DIRECTION_TOLERANCE {
        Ok(())
    } else {
        Err(AlignmentError::SemanticViolation {
            entity: Some(next.entity),
            rule: "consecutive horizontal segments must share a tangent direction: \
                   one exact plan curve cannot carry a kink",
        })
    }
}

/// The unit frame at the origin. Turning does not depend on placement, so
/// any frame serves to ask the kernel for a law's closed-form integral.
fn unit_frame() -> Frame2 {
    Frame2 {
        origin: Point2::new(0.0, 0.0),
        x: Vec2::new(1.0, 0.0),
        y: Vec2::new(0.0, 1.0),
    }
}

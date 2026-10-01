//! Cant as data at a station: rail heights, bank angle and the section frame.
//!
//! The cant layout states, per station, how far each rail head sits above
//! the vertical profile (`IfcAlignmentCantSegment` "measured relatively to
//! vertical alignment"). With the parent's `RailHeadDistance` `b` that fixes
//! the cross-section exactly:
//!
//! - cant `D = left - right`, positive when the left rail is higher;
//! - bank angle `psi = arcsin(D / b)`, the relation IFC4.3 ADD2
//!   (`IfcAlignmentCantSegmentTypeEnum`) states, so the two rail heads sit
//!   `b/2 sin(psi) = D/2` above and below the section's rotation point;
//! - that point -- the deviating elevation `IfcSegmentedReferenceCurve`
//!   describes -- lies `(left + right) / 2` above the profile: zero for a
//!   rotation about the track centreline, `D/2` for one about the low rail.
//!
//! The frame is the section rotated by `psi` about the centreline tangent,
//! right-handed: in the basis `(t, n, u)` -- `t` the unit 3D tangent of the
//! gradient curve, `n` the horizontal unit normal to its left, `u = t x n`
//! -- the rail-to-rail axis is `cos(psi) n + sin(psi) u` and the section's
//! up axis `-sin(psi) n + cos(psi) u`.
//!
//! This is data, not lowered geometry: the neutral curve vocabulary has no
//! roll law to attach it to (see `lower_segmented_reference_curve`), and the
//! tangent comes from whichever evaluator the caller uses. [`CantFrame::orient`]
//! turns a caller-supplied point and tangent into a world frame; it is
//! algebra on those vectors, not evaluation of any curve.

use axiolid_core::{Frame3, Point3, Vec2, Vec3};

use crate::cant::layout::CantLayout;
use crate::error::{AlignmentError, AlignmentResult};

/// The cant cross-section at one station.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CantFrame {
    /// Absolute distance along the horizontal alignment, in metres.
    pub distance_along: f64,
    /// Left rail head above the vertical profile, in metres.
    pub left: f64,
    /// Right rail head above the vertical profile, in metres.
    pub right: f64,
    /// Cant `D = left - right`, positive when the left rail is higher.
    pub cant: f64,
    /// Elevation of the section's rotation point above the vertical
    /// profile: `(left + right) / 2`.
    pub axis_elevation: f64,
    /// Bank angle `psi = arcsin(D / b)` in radians, positive raising the
    /// left rail (counter-clockwise looking along the tangent).
    pub bank_angle: f64,
    /// Rail-to-rail axis (towards the left rail) as `(n, u)` components.
    pub lateral: Vec2,
    /// Section up axis as `(n, u)` components.
    pub up: Vec2,
}

impl CantFrame {
    /// The section frame in world coordinates.
    ///
    /// `point` and `tangent` are the gradient curve's point and 3D tangent
    /// at this frame's station, from the caller's evaluator. The origin is
    /// `point` raised by [`Self::axis_elevation`] along world up (the cant
    /// is measured vertically from the profile); `x` is the unit tangent,
    /// `y` the rail-to-rail axis and `z` the section up axis.
    ///
    /// # Errors
    ///
    /// Refuses a non-finite point or tangent, and a vertical or zero
    /// tangent, which has no horizontal left normal.
    pub fn orient(&self, point: Point3, tangent: Vec3) -> AlignmentResult<Frame3> {
        let invalid = |detail| Err(AlignmentError::InvalidUnits { detail });
        if !(point.is_finite() && tangent.is_finite()) {
            return invalid("cant frame point and tangent must be finite");
        }
        let horizontal = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
        if horizontal == 0.0 {
            return invalid("a vertical or zero tangent has no horizontal left normal");
        }
        let t = tangent.normalize();
        let n = Vec3::new(-tangent.y / horizontal, tangent.x / horizontal, 0.0);
        let u = t.cross(n);
        Ok(Frame3 {
            origin: point + Vec3::Z * self.axis_elevation,
            x: t,
            y: n * self.lateral.x + u * self.lateral.y,
            z: n * self.up.x + u * self.up.y,
        })
    }
}

impl CantLayout {
    /// The cant cross-section at an absolute distance along.
    ///
    /// # Errors
    ///
    /// Refuses a distance outside the profile's span, a segment whose cant
    /// cannot be evaluated exactly, and a cant exceeding the
    /// `RailHeadDistance` (`|D| > b`, no real bank angle).
    pub fn frame_at_distance(&self, distance_along: f64) -> AlignmentResult<CantFrame> {
        let at = self.cant_at_distance(distance_along)?;
        let cant = at.left - at.right;
        let ratio = cant / self.rail_head_distance;
        if !(-1.0..=1.0).contains(&ratio) {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(self.entity),
                rule: "cant must not exceed the rail head distance (|D| <= b)",
            });
        }
        let bank_angle = ratio.asin();
        let (sin, cos) = bank_angle.sin_cos();
        Ok(CantFrame {
            distance_along,
            left: at.left,
            right: at.right,
            cant,
            axis_elevation: 0.5 * (at.left + at.right),
            bank_angle,
            lateral: Vec2::new(cos, sin),
            up: Vec2::new(-sin, cos),
        })
    }
}

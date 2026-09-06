//! Transition spirals lowered exactly as intrinsic (natural-equation) curves.
//!
//! A transition spiral has no elementary parametric form: its Cartesian
//! position is a Fresnel-type integral. Its *curvature*, however, is an
//! elementary closed-form function of arc length, and a plane curve is fixed
//! up to rigid motion by that law. Anchoring the law to a start frame fixes
//! it absolutely.
//!
//! So the exact lowering is `Curve2::Intrinsic`: store the curvature law and
//! the start frame, integrate nothing. This is a lossless representation, not
//! an approximation -- no quadrature, no series, no sampling.
//!
//! # Where the laws come from
//!
//! IFC4X3 states each spiral's curvature law normatively on the geometry
//! entity itself. `IfcAlignmentHorizontalSegment` carries only endpoint radii
//! and a length, so the law is reconstructed from those: every family below is
//! pinned by the boundary conditions k(0) = 1/R_start and k(L) = 1/R_end.
//!
//! A zero radius means "no curvature" (straight) in IFC's alignment
//! convention, not an infinitely tight curve -- `curvature_of` encodes that.

use axiolid_core::{Frame2, Vec2};
use axiolid_curve::{CurvatureLaw, Curve2, Intrinsic2};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::HorizontalSegment;

/// IFC alignment radius convention: 0 denotes a straight (zero curvature).
fn curvature_of(radius: f64) -> f64 {
    if radius == 0.0 {
        0.0
    } else {
        1.0 / radius
    }
}

/// The exact curvature law for a transition family, from its endpoint
/// curvatures and length.
///
/// `None` when the family is not a transition spiral this function models.
/// Every returned law satisfies k(0) = `start` and k(L) = `end` exactly.
fn transition_law(name: &str, start: f64, end: f64, length: f64) -> Option<CurvatureLaw> {
    let delta = end - start;
    match name {
        // Curvature linear in arc length: k(s) = k0 + (k1-k0)(s/L).
        // This is the clothoid/Euler spiral, IFC's CLOTHOID.
        "CLOTHOID" => Some(CurvatureLaw::clothoid(start, end, length)),

        // Bloss: k(s) = k0 + delta*(3u^2 - 2u^3), u = s/L. Expanded in s:
        //   k(s) = k0 + (3*delta/L^2) s^2 - (2*delta/L^3) s^3.
        "BLOSSCURVE" => Some(CurvatureLaw::Polynomial {
            coefficients: vec![
                start,
                0.0,
                3.0 * delta / (length * length),
                -2.0 * delta / (length * length * length),
            ],
        }),

        // Cosine: k(s) = k0 + (delta/2)(1 - cos(pi s / L)).
        // The kernel's Sinusoid is `mean + A sin(w s + phase)`, so the cosine
        // is folded in via sin(x - pi/2) = -cos(x): mean k0 + delta/2,
        // amplitude delta/2, phase -pi/2. Exact -- a truncated series is not.
        "COSINECURVE" => Some(CurvatureLaw::Sinusoid {
            mean: start + delta / 2.0,
            amplitude: delta / 2.0,
            angular_frequency: core::f64::consts::PI / length,
            phase: -core::f64::consts::FRAC_PI_2,
        }),

        _ => None,
    }
}

/// Whether this crate can lower the named transition family exactly.
pub fn is_exactly_lowerable(name: &str) -> bool {
    matches!(name, "CLOTHOID" | "BLOSSCURVE" | "COSINECURVE")
}

/// Lower a transition-spiral segment to an exact intrinsic curve.
///
/// Refuses rather than approximates when the family has no single closed-form
/// curvature law in this vocabulary (`HELMERTCURVE` is piecewise, `SINECURVE`
/// and `VIENNESEBEND` need terms this crate does not reconstruct from
/// endpoint radii alone).
pub fn spiral_curve(segment: &HorizontalSegment, name: &str) -> AlignmentResult<Curve2> {
    if !(segment.segment_length.is_finite() && segment.segment_length > 0.0) {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "a transition spiral requires a finite, positive segment length",
        });
    }
    let start = curvature_of(segment.start_radius);
    let end = curvature_of(segment.end_radius);
    if !start.is_finite() || !end.is_finite() {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "transition spiral endpoint curvatures must be finite",
        });
    }
    let law = transition_law(name, start, end, segment.segment_length).ok_or_else(|| {
        AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: name.to_owned(),
            detail: "no single closed-form curvature law reconstructs this family \
                     from endpoint radii alone",
        }
    })?;
    let direction = Vec2::new(segment.start_direction.cos(), segment.start_direction.sin());
    let frame = Frame2 {
        origin: segment.start_point,
        x: direction,
        y: Vec2::new(-direction.y, direction.x),
    };
    let curve = Intrinsic2::new(frame, law, segment.segment_length);
    // The turning integral is closed form for every law above; a non-finite
    // result means the reconstructed law is degenerate, which is a refusal
    // rather than something to hand downstream.
    if curve.total_turning().is_none_or(|turn| !turn.is_finite()) {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "transition spiral turning integral is not finite",
        });
    }
    Ok(Curve2::Intrinsic(curve))
}

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

use crate::cant::CantLayout;
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

        // Helmert (a.k.a. Schramm): two quadratic halves meeting at the
        // midpoint, each exactly quadratic in arc length.
        //   first  half, u <= 1/2: k = k0 + 2 u^2 delta
        //   second half, u >  1/2: k = k0 + (1 - 2(1-u)^2) delta
        // Expanded in s (u = s/L), the first half is [k0, 0, 2 delta / L^2].
        // Pieces are rebased: a piece's law is evaluated in its OWN arc length
        // measured from its start, so the second half is written in
        // t = s - L/2 and becomes [k0 + delta/2, 2 delta / L, -2 delta / L^2].
        // The seam is C1 -- both halves have slope 2 delta / L there -- so the
        // break is a change of formula, not of the curve.
        //
        // A piecewise law must stay inside ONE curve: a separate curve per
        // half would need an absolute start frame at the seam, whose origin
        // is the position there -- a non-elementary integral this crate
        // refuses to compute. Anchoring the interior by arc length alone
        // keeps the lowering exact.
        "HELMERTCURVE" => Some(CurvatureLaw::piecewise(
            vec![length / 2.0],
            vec![
                CurvatureLaw::Polynomial {
                    coefficients: vec![start, 0.0, 2.0 * delta / (length * length)],
                },
                CurvatureLaw::Polynomial {
                    coefficients: vec![
                        start + delta / 2.0,
                        2.0 * delta / length,
                        -2.0 * delta / (length * length),
                    ],
                },
            ],
        )),

        // Sine: k(s) = k0 + delta*(u - sin(2 pi u)/(2 pi)), a linear ramp with
        // one full sine correction. Needs a polynomial AND a harmonic term at
        // once, which the kernel models directly.
        "SINECURVE" => Some(CurvatureLaw::sine_corrected_transition(start, end, length)),

        _ => None,
    }
}

/// The Viennese bend curvature law, exactly.
///
/// IFC 4.3 defines it as a 7th order polynomial spiral. The endpoint
/// curvature ramp uses the shape function
///
/// ```text
/// f(xi) = xi^4 * (35 - 84 xi + 70 xi^2 - 20 xi^3)
/// ```
///
/// and the cant correction is the second derivative of that same
/// function, scaled by -h * dpsi, with xi = s / L. Expanded in arc
/// length the whole law is a degree-7 polynomial, which
/// `CurvatureLaw::Polynomial` represents exactly: no truncation, no
/// fitting.
///
/// `dpsi` is the superelevation swing across the segment -- the change
/// in roll angle the cant produces -- and `h` is the gravity centre line
/// height, which converts that roll angle into a curvature correction.
fn viennese_law(
    start: f64,
    end: f64,
    length: f64,
    gravity_height: f64,
    roll_swing: f64,
) -> CurvatureLaw {
    let delta = end - start;
    let l = length;
    // Coefficients in ascending powers of arc length, from expanding
    // the xi-form above. Written out rather than built by a loop so the
    // degree-7 shape is visible and checkable against the spec.
    let hp = gravity_height * roll_swing;
    let c2 = -420.0 * hp / l.powi(4);
    let c3 = 1680.0 * hp / l.powi(5);
    let c4 = 35.0 * (l * l * delta - 60.0 * hp) / l.powi(6);
    let c5 = 84.0 * (-(l * l) * delta + 10.0 * hp) / l.powi(7);
    let c6 = 70.0 * delta / l.powi(6);
    let c7 = -20.0 * delta / l.powi(7);
    CurvatureLaw::Polynomial {
        coefficients: vec![start, 0.0, c2, c3, c4, c5, c6, c7],
    }
}

/// Whether this crate can lower the named transition family exactly.
pub fn is_exactly_lowerable(name: &str, has_cant: bool) -> bool {
    match name {
        "CLOTHOID" | "BLOSSCURVE" | "COSINECURVE" | "HELMERTCURVE" | "SINECURVE" => true,
        // The Viennese bend is exactly representable, but only when the
        // cant swing and gravity centre line height it depends on are
        // present. Without them the law is undetermined, not approximate.
        "VIENNESEBEND" => has_cant,
        _ => false,
    }
}

/// Lower a transition-spiral segment to an exact intrinsic curve.
///
/// Refuses rather than approximates when the family's law cannot be
/// reconstructed from what the file states.
///
/// Most families need only the endpoint radii and the length. `VIENNESEBEND`
/// additionally needs the cant swing and the gravity centre line height, so
/// `cant` is not optional for it: a horizontal segment alone does not
/// determine that geometry. Passing `None` for a Viennese bend is a refusal,
/// not a fallback to an approximate law.
pub fn spiral_curve(
    segment: &HorizontalSegment,
    name: &str,
    cant: Option<&CantLayout>,
    start_distance: f64,
) -> AlignmentResult<Curve2> {
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
    let law = if name == "VIENNESEBEND" {
        viennese_curvature(segment, start, end, cant, start_distance)?
    } else {
        transition_law(name, start, end, segment.segment_length).ok_or_else(|| {
            AlignmentError::Unsupported {
                entity: segment.entity,
                type_name: name.to_owned(),
                detail: "no single closed-form curvature law reconstructs this family \
                         from endpoint radii alone",
            }
        })?
    };
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

/// Resolve the Viennese bend law from the segment and its cant layout.
///
/// Both inputs the law needs beyond the endpoint radii are refused by
/// name when absent, never defaulted. A missing gravity centre line
/// height is not zero: zero places the centre of mass on the rail plane,
/// which is a different curve rather than an unknown one.
fn viennese_curvature(
    segment: &HorizontalSegment,
    start: f64,
    end: f64,
    cant: Option<&CantLayout>,
    start_distance: f64,
) -> AlignmentResult<CurvatureLaw> {
    let Some(layout) = cant else {
        return Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: "VIENNESEBEND".to_owned(),
            detail: "a Viennese bend needs the IfcAlignmentCant layout: its \\
                     curvature law depends on the superelevation swing",
        });
    };
    let Some(height) = segment.gravity_center_line_height else {
        return Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: "VIENNESEBEND".to_owned(),
            detail: "a Viennese bend needs GravityCenterLineHeight: without \\
                     it the cant correction is undetermined, not zero",
        });
    };
    if !height.is_finite() {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "GravityCenterLineHeight must be finite",
        });
    }
    let swing = roll_swing(segment, layout, start_distance)?;
    Ok(viennese_law(
        start,
        end,
        segment.segment_length,
        height,
        swing,
    ))
}

/// The change in superelevation roll angle across the segment.
///
/// Cant is an elevation difference between the rails; the roll angle it
/// produces is `asin(D / b)` for rail head distance `b`. The swing is
/// the difference between the angles at the segment end and start, which
/// is what the curvature correction scales.
///
/// Stations come from the caller because an IfcAlignmentHorizontalSegment
/// does not state its own distance along: position follows from chaining.
fn roll_swing(
    segment: &HorizontalSegment,
    layout: &CantLayout,
    start_distance: f64,
) -> AlignmentResult<f64> {
    if !start_distance.is_finite() || start_distance < 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "a Viennese bend needs a finite, non-negative station",
        });
    }
    let end_distance = start_distance + segment.segment_length;
    let at_start = layout.cant_at_distance(start_distance)?;
    let at_end = layout.cant_at_distance(end_distance)?;
    let b = layout.rail_head_distance;
    if !(b.is_finite() && b > 0.0) {
        return Err(AlignmentError::InvalidSegment {
            entity: layout.entity,
            detail: "RailHeadDistance must be finite and positive to convert \\
                     cant into a roll angle",
        });
    }
    let angle = |station: &crate::cant::CantAtStation| {
        // Cant is the elevation difference between the rails, so the roll
        // is taken from that difference rather than either rail alone.
        let d = station.left - station.right;
        (d / b).asin()
    };
    let swing = angle(&at_end) - angle(&at_start);
    if !swing.is_finite() {
        return Err(AlignmentError::InvalidSegment {
            entity: layout.entity,
            detail: "the cant swing across this segment is not finite",
        });
    }
    Ok(swing)
}

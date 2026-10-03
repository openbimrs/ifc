//! The IFC4X3 `IfcSpiral` subtypes as exact curvature laws.
//!
//! # Natural equations, never positions
//!
//! A spiral is `lambda(u) = C + int_0^u cos(theta) x + int_0^u sin(theta) y`
//! (`IfcSpiral`), its heading `theta` an elementary function of the arc
//! length. The position is a Fresnel-type integral with no closed form, so a
//! spiral lowers as its curvature law `k = dtheta/ds` on a
//! `Curve3::Intrinsic` and is never integrated here.
//!
//! # The laws (IFC4.3 ADD2, buildingSMART IFC4.3.x-development)
//!
//! With every term a length `A_n`, the polynomial spirals state
//!
//! ```text
//! k(s) = A_7 s^7/|A_7|^9 + s^6/A_6^7 + A_5 s^5/|A_5|^7 + s^4/A_4^5
//!      + A_3 s^3/|A_3|^5 + s^2/A_2^3 + A_1 s/|A_1|^3 + 1/A_0
//! ```
//!
//! (figures `ifcsecondorderpolynomialspiral_curvature`, `..third..`,
//! `..seventh..`). Every term is `sign(A_n) s^n / |A_n|^(n+1)`, so it is one
//! rule for every degree. `IfcClothoid` is the degree-1 case,
//! `k = A s / |A^3|`. An absent term contributes nothing; a PRESENT zero term
//! is an infinite coefficient and is refused, never read as absent.
//!
//! `IfcCosineSpiral` and `IfcSineSpiral` state their heading, with `L` the
//! length of the using segment ("terms dependent on segment length L",
//! concept templates *Cosine/Sine Spiral Transition Segment*):
//!
//! ```text
//! cosine: theta = s/A_0 + L/(pi A_1) sin(pi s / L)
//! sine:   theta = s/A_0 + sign(A_1)/2 (s/|A_1|)^2 - L/(2 pi A_2)(cos(2 pi s/L) - 1)
//! ```
//!
//! so `k = 1/A_0 + cos(pi s/L)/A_1` and `k = 1/A_0 + sign(A_1) s/A_1^2 +
//! sin(2 pi s/L)/A_2`. The published `kappa` lines carry an extra factor `L`:
//! they are `dtheta/d(s/L)`, the derivative in the normalised length; the
//! heading is the definition used. Because the law depends on `L`, these two
//! are determined only inside an `IfcCurveSegment`.
//!
//! # Units
//!
//! Every term is an `IfcLengthMeasure` and is converted to metres before the
//! law is formed, so the coefficients are in metre powers.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use axiolid_curve::{CurvatureLaw, Harmonic};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

/// Why a spiral met on its own is refused.
///
/// [`crate::lower::dispatch::PLANNED`] quotes it verbatim for the polynomial
/// spirals; `tests/lower_dispatch_corpus.rs` holds the two equal.
pub(crate) const STANDALONE_SPIRAL: &str =
    "an IfcSpiral is unbounded (-inf < u < inf) and the neutral intrinsic curve needs a \
     finite arc length; it lowers exactly as the ParentCurve of an IfcCurveSegment";

/// Why a cosine or sine spiral met on its own is refused.
pub(crate) const STANDALONE_HARMONIC_SPIRAL: &str =
    "an IfcCosineSpiral or IfcSineSpiral law depends on the length L of the IfcCurveSegment \
     using it; it lowers exactly only as the ParentCurve of an IfcCurveSegment";

/// Is `kind` one of the six `IfcSpiral` subtypes?
pub(crate) fn is_spiral(kind: &str) -> bool {
    matches!(
        kind,
        "IFCCLOTHOID"
            | "IFCCOSINESPIRAL"
            | "IFCSINESPIRAL"
            | "IFCSECONDORDERPOLYNOMIALSPIRAL"
            | "IFCTHIRDORDERPOLYNOMIALSPIRAL"
            | "IFCSEVENTHORDERPOLYNOMIALSPIRAL"
    )
}

/// The reason a standalone spiral of `kind` is refused.
pub(crate) fn standalone_reason(kind: &str) -> &'static str {
    match kind {
        "IFCCOSINESPIRAL" | "IFCSINESPIRAL" => STANDALONE_HARMONIC_SPIRAL,
        _ => STANDALONE_SPIRAL,
    }
}

/// The exact curvature law of spiral `id` in its own arc length from `C`.
///
/// `segment_length` is the absolute length `L` of the using segment, in
/// metres; only the cosine and sine spirals read it.
pub(crate) fn spiral_law(
    session: &LoweringSession<'_>,
    id: EntityId,
    kind: &str,
    segment_length: f64,
) -> GeometryResult<CurvatureLaw> {
    let slots = session.slots(id)?;
    // Slot 0 is the inherited `Position`; the terms follow, highest degree
    // first, exactly as `IFC4X3_ADD2.exp` lists them.
    let term = |index: usize, name: &'static str, required: bool| -> GeometryResult<Option<f64>> {
        let raw = if required {
            Some(slots.req_f64(index, name)?)
        } else {
            slots.opt_f64(index)
        };
        let Some(raw) = raw else { return Ok(None) };
        let metres = session.units().length(raw);
        if !metres.is_finite() || metres == 0.0 {
            return Err(session.degenerate(
                id,
                kind,
                format!("{name} must be finite and non-zero: a zero term is an infinite coefficient, not an absent one"),
            ));
        }
        Ok(Some(metres))
    };
    let polynomial = |terms: &[(usize, &'static str, bool)]| -> GeometryResult<CurvatureLaw> {
        // `terms` runs from the highest degree down to the constant.
        let degree = terms.len() - 1;
        let mut coefficients = vec![0.0; degree + 1];
        for (offset, (index, name, required)) in terms.iter().enumerate() {
            if let Some(a) = term(*index, name, *required)? {
                let power = degree - offset;
                coefficients[power] = polynomial_term(a, power);
            }
        }
        Ok(CurvatureLaw::Polynomial { coefficients })
    };

    let law = match kind {
        // Degree 1 with no lower terms: `k = A s / |A^3|`.
        "IFCCLOTHOID" => {
            let a = term(1, "ClothoidConstant", true)?.unwrap_or(f64::NAN);
            CurvatureLaw::Polynomial {
                coefficients: vec![0.0, polynomial_term(a, 1)],
            }
        }
        "IFCSECONDORDERPOLYNOMIALSPIRAL" => polynomial(&[
            (1, "QuadraticTerm", true),
            (2, "LinearTerm", false),
            (3, "ConstantTerm", false),
        ])?,
        "IFCTHIRDORDERPOLYNOMIALSPIRAL" => polynomial(&[
            (1, "CubicTerm", true),
            (2, "QuadraticTerm", false),
            (3, "LinearTerm", false),
            (4, "ConstantTerm", false),
        ])?,
        "IFCSEVENTHORDERPOLYNOMIALSPIRAL" => polynomial(&[
            (1, "SepticTerm", true),
            (2, "SexticTerm", false),
            (3, "QuinticTerm", false),
            (4, "QuarticTerm", false),
            (5, "CubicTerm", false),
            (6, "QuadraticTerm", false),
            (7, "LinearTerm", false),
            (8, "ConstantTerm", false),
        ])?,
        "IFCCOSINESPIRAL" => {
            let cosine = term(1, "CosineTerm", true)?.unwrap_or(f64::NAN);
            let constant = term(2, "ConstantTerm", false)?;
            harmonic_law(
                session,
                id,
                kind,
                segment_length,
                vec![constant.map_or(0.0, |a| polynomial_term(a, 0))],
                // cos(x) = sin(x + pi/2).
                1.0 / cosine,
                PI,
                FRAC_PI_2,
            )?
        }
        "IFCSINESPIRAL" => {
            let sine = term(1, "SineTerm", true)?.unwrap_or(f64::NAN);
            let linear = term(2, "LinearTerm", false)?;
            let constant = term(3, "ConstantTerm", false)?;
            harmonic_law(
                session,
                id,
                kind,
                segment_length,
                vec![
                    constant.map_or(0.0, |a| polynomial_term(a, 0)),
                    linear.map_or(0.0, |a| polynomial_term(a, 1)),
                ],
                1.0 / sine,
                TAU,
                0.0,
            )?
        }
        _ => unreachable!("spiral_law is called only for IfcSpiral subtypes"),
    };
    if !law_is_finite(&law) {
        return Err(session.degenerate(id, kind, "the curvature law's coefficients overflow f64"));
    }
    Ok(law)
}

/// `sign(A) / |A|^(n+1)`: the degree-`n` coefficient a term `A` states.
fn polynomial_term(a: f64, power: usize) -> f64 {
    let exponent = i32::try_from(power + 1).unwrap_or(i32::MAX);
    a.signum() / a.abs().powi(exponent)
}

/// A polynomial plus one harmonic of period `period_factor`-th of `L`.
#[allow(clippy::too_many_arguments)]
fn harmonic_law(
    session: &LoweringSession<'_>,
    id: EntityId,
    kind: &str,
    segment_length: f64,
    polynomial: Vec<f64>,
    amplitude: f64,
    turn: f64,
    phase: f64,
) -> GeometryResult<CurvatureLaw> {
    if !(segment_length.is_finite() && segment_length > 0.0) {
        return Err(session.degenerate(
            id,
            kind,
            "the using IfcCurveSegment must have a finite, non-zero length L",
        ));
    }
    Ok(CurvatureLaw::Composite {
        polynomial,
        harmonics: vec![Harmonic {
            amplitude,
            angular_frequency: turn / segment_length,
            phase,
        }],
    })
}

/// Every coefficient, amplitude, frequency and phase is finite.
fn law_is_finite(law: &CurvatureLaw) -> bool {
    match law {
        CurvatureLaw::Polynomial { coefficients } => coefficients.iter().all(|c| c.is_finite()),
        CurvatureLaw::Composite {
            polynomial,
            harmonics,
        } => {
            polynomial.iter().all(|c| c.is_finite())
                && harmonics.iter().all(|h| {
                    h.amplitude.is_finite()
                        && h.angular_frequency.is_finite()
                        && h.phase.is_finite()
                })
        }
        _ => false,
    }
}

/// The law of the piece `[start, start + length]` of `law`, in the piece's
/// own arc length, traversed in the sense of `length`'s sign.
///
/// Forward, the piece's curvature is `k(start + t)`. Backward (a negative
/// `SegmentLength`, "the sign of this value defines the sense agreement"),
/// the curve is walked from `start` towards smaller `s`; reversing a plane
/// curve flips its tangent and so its signed curvature, giving
/// `-k(start - t)`. Both are closed-form rewrites: a binomial shift of the
/// polynomial and a phase shift of each harmonic.
pub(crate) fn piece_law(law: &CurvatureLaw, start: f64, length: f64) -> Option<CurvatureLaw> {
    if length >= 0.0 {
        return law.shifted(start);
    }
    Some(reflected(law)?.shifted(-start)?.reversed_orientation())
}

/// `k(-u)`: the law read with its arc length negated.
fn reflected(law: &CurvatureLaw) -> Option<CurvatureLaw> {
    let odd = |power: usize, c: f64| if power % 2 == 1 { -c } else { c };
    // a sin(-w u + p) = -a sin(w u - p).
    let harmonic = |h: &Harmonic| Harmonic {
        amplitude: -h.amplitude,
        angular_frequency: h.angular_frequency,
        phase: -h.phase,
    };
    match law {
        CurvatureLaw::Constant { curvature } => Some(CurvatureLaw::Constant {
            curvature: *curvature,
        }),
        CurvatureLaw::Polynomial { coefficients } => Some(CurvatureLaw::Polynomial {
            coefficients: coefficients
                .iter()
                .enumerate()
                .map(|(power, c)| odd(power, *c))
                .collect(),
        }),
        CurvatureLaw::Composite {
            polynomial,
            harmonics,
        } => Some(CurvatureLaw::Composite {
            polynomial: polynomial
                .iter()
                .enumerate()
                .map(|(power, c)| odd(power, *c))
                .collect(),
            harmonics: harmonics.iter().map(harmonic).collect(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

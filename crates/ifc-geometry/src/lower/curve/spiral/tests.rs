//! Spiral laws against hand-computed coefficients.
//!
//! Every expected coefficient below is `sign(A) / |A|^(n+1)` worked by hand
//! from the stated term, so a sign or exponent slip in the rule fails here.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use axiolid_curve::{CurvatureLaw, Harmonic};
use ifc_model::{EntityId, Model, Value};

use super::{piece_law, spiral_law};
use crate::lower::session::LoweringSession;
use crate::solid::testkit::{entity, n};
use crate::units::UnitScale;

const METRES: UnitScale = UnitScale {
    length_to_metres: 1.0,
    angle_to_radians: 1.0,
};

/// A model whose `#1` is `kind` with the given term slots after `Position`.
fn spiral(kind: &str, terms: &[Option<f64>]) -> Model {
    let mut attributes = vec![Value::Null];
    attributes.extend(terms.iter().map(|t| t.map_or(Value::Null, n)));
    let mut model = Model::new();
    model.insert(EntityId(1), entity(kind, attributes));
    model
}

fn law(model: &Model, kind: &str, units: &UnitScale, length: f64) -> CurvatureLaw {
    let session = LoweringSession::new(model, units);
    spiral_law(&session, EntityId(1), kind, length).expect("law")
}

fn assert_close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() <= 1e-12 * e.abs().max(1e-300),
            "{actual:?} vs {expected:?}"
        );
    }
}

fn polynomial(law: &CurvatureLaw) -> &[f64] {
    match law {
        CurvatureLaw::Polynomial { coefficients } => coefficients,
        other => panic!("expected a polynomial law, got {other:?}"),
    }
}

/// Closed-form evaluation of the two law shapes this module produces.
fn evaluate(law: &CurvatureLaw, s: f64) -> f64 {
    let poly = |c: &[f64]| c.iter().rev().fold(0.0, |acc, c| acc * s + c);
    match law {
        CurvatureLaw::Polynomial { coefficients } => poly(coefficients),
        CurvatureLaw::Composite {
            polynomial,
            harmonics,
        } => {
            poly(polynomial)
                + harmonics
                    .iter()
                    .map(|h| h.amplitude * (h.angular_frequency * s + h.phase).sin())
                    .sum::<f64>()
        }
        other => panic!("unexpected law {other:?}"),
    }
}

/// `k = A s / |A^3|`: A = 100 m gives 1e-4 per metre, sign from A.
#[test]
fn a_clothoid_is_linear_curvature_with_the_sign_of_its_constant() {
    let left = law(
        &spiral("IFCCLOTHOID", &[Some(100.0)]),
        "IFCCLOTHOID",
        &METRES,
        1.0,
    );
    assert_close(polynomial(&left), &[0.0, 1e-4]);
    let right = law(
        &spiral("IFCCLOTHOID", &[Some(-100.0)]),
        "IFCCLOTHOID",
        &METRES,
        1.0,
    );
    assert_close(polynomial(&right), &[0.0, -1e-4]);
}

/// Terms are lengths: 100000 mm is 100 m, not 100000 m.
#[test]
fn spiral_terms_convert_to_metres_before_the_law_is_formed() {
    let millimetres = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let law = law(
        &spiral("IFCCLOTHOID", &[Some(100_000.0)]),
        "IFCCLOTHOID",
        &millimetres,
        1.0,
    );
    assert_close(polynomial(&law), &[0.0, 1e-4]);
}

/// `s^2/A_2^3 + A_1 s/|A_1^3| + 1/A_0` with 150, 200, 400 m.
#[test]
fn a_second_order_spiral_reads_each_term_by_its_own_degree() {
    let model = spiral(
        "IFCSECONDORDERPOLYNOMIALSPIRAL",
        &[Some(150.0), Some(200.0), Some(400.0)],
    );
    let law = law(&model, "IFCSECONDORDERPOLYNOMIALSPIRAL", &METRES, 1.0);
    assert_close(
        polynomial(&law),
        &[1.0 / 400.0, 1.0 / 40_000.0, 1.0 / 3_375_000.0],
    );
}

/// An absent term contributes nothing; the leading cubic is `A_3 s^3/|A_3^5|`.
#[test]
fn a_third_order_spiral_with_only_its_cubic_term() {
    let model = spiral(
        "IFCTHIRDORDERPOLYNOMIALSPIRAL",
        &[Some(-120.0), None, None, None],
    );
    let law = law(&model, "IFCTHIRDORDERPOLYNOMIALSPIRAL", &METRES, 1.0);
    assert_close(polynomial(&law), &[0.0, 0.0, 0.0, -1.0 / 207_360_000.0]);
}

/// Even exponents keep the sign: `s^6/A_6^7` with A_6 = -50 is negative.
#[test]
fn a_seventh_order_spiral_keeps_every_terms_sign() {
    let model = spiral(
        "IFCSEVENTHORDERPOLYNOMIALSPIRAL",
        &[
            Some(100.0),
            Some(-50.0),
            None,
            None,
            None,
            None,
            None,
            Some(-250.0),
        ],
    );
    let law = law(&model, "IFCSEVENTHORDERPOLYNOMIALSPIRAL", &METRES, 1.0);
    assert_close(
        polynomial(&law),
        &[
            -1.0 / 250.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0 / 7.8125e11,
            1.0 / 1e16,
        ],
    );
}

/// `k = 1/A_0 + cos(pi s / L)/A_1`, the cosine folded into a sine.
#[test]
fn a_cosine_spiral_depends_on_the_segment_length() {
    let model = spiral("IFCCOSINESPIRAL", &[Some(200.0), Some(400.0)]);
    let law = law(&model, "IFCCOSINESPIRAL", &METRES, 50.0);
    assert_eq!(
        law,
        CurvatureLaw::Composite {
            polynomial: vec![1.0 / 400.0],
            harmonics: vec![Harmonic {
                amplitude: 1.0 / 200.0,
                angular_frequency: PI / 50.0,
                phase: FRAC_PI_2,
            }],
        }
    );
}

/// `k = 1/A_0 + sign(A_1) s/A_1^2 + sin(2 pi s / L)/A_2`.
#[test]
fn a_sine_spiral_carries_a_ramp_and_one_full_sine() {
    let model = spiral("IFCSINESPIRAL", &[Some(200.0), Some(300.0), None]);
    let law = law(&model, "IFCSINESPIRAL", &METRES, 50.0);
    assert_eq!(
        law,
        CurvatureLaw::Composite {
            polynomial: vec![0.0, 1.0 / 90_000.0],
            harmonics: vec![Harmonic {
                amplitude: 1.0 / 200.0,
                angular_frequency: TAU / 50.0,
                phase: 0.0,
            }],
        }
    );
}

/// IfcOpenShell writes a Bloss transition (R = inf to 200 m over 60 m) as a
/// third-order spiral with `A_n = L |a_n|^(-1/(n+1)) sign(a_n)`, a_2 = 3f,
/// a_3 = -2f, f = L/R. Read back, that is the Bloss law
/// `k = 3 d s^2 / L^2 - 2 d s^3 / L^3` with d = 1/200: k(0) = 0, k(L) = 1/R.
#[test]
fn exporter_bloss_terms_read_back_as_the_bloss_law() {
    let (length, radius) = (60.0_f64, 200.0_f64);
    let f = length / radius;
    let term = |a: f64, power: i32| length * a.abs().powf(-1.0 / f64::from(power + 1)) * a.signum();
    let model = spiral(
        "IFCTHIRDORDERPOLYNOMIALSPIRAL",
        &[Some(term(-2.0 * f, 3)), Some(term(3.0 * f, 2)), None, None],
    );
    let law = law(&model, "IFCTHIRDORDERPOLYNOMIALSPIRAL", &METRES, length);
    let d = 1.0 / radius;
    let expected = [
        0.0,
        0.0,
        3.0 * d / length.powi(2),
        -2.0 * d / length.powi(3),
    ];
    for (a, e) in polynomial(&law).iter().zip(expected) {
        assert!((a - e).abs() <= 1e-15, "{a} vs {e}");
    }
    assert!((evaluate(&law, length) - d).abs() <= 1e-15);
}

/// IfcOpenShell's cosine terms (A_0 = L/a_0, A_1 = L/a_1, a_0 = f/2,
/// a_1 = -f/2) reproduce k(0) = 0 and k(L) = 1/R.
#[test]
fn exporter_cosine_terms_ramp_between_the_end_curvatures() {
    let (length, radius) = (50.0, 200.0);
    let f = length / radius;
    let model = spiral(
        "IFCCOSINESPIRAL",
        &[Some(length / (-0.5 * f)), Some(length / (0.5 * f))],
    );
    let law = law(&model, "IFCCOSINESPIRAL", &METRES, length);
    assert!(evaluate(&law, 0.0).abs() <= 1e-15);
    assert!((evaluate(&law, length) - 1.0 / radius).abs() <= 1e-15);
}

/// A PRESENT zero term is an infinite coefficient, not an absent term.
#[test]
fn a_zero_term_is_refused_rather_than_read_as_absent() {
    let model = spiral(
        "IFCSECONDORDERPOLYNOMIALSPIRAL",
        &[Some(150.0), Some(0.0), None],
    );
    let session = LoweringSession::new(&model, &METRES);
    let error = spiral_law(&session, EntityId(1), "IFCSECONDORDERPOLYNOMIALSPIRAL", 1.0)
        .expect_err("a zero term has no finite coefficient");
    assert!(
        !error.is_unsupported(),
        "this is bad data, not a gap: {error}"
    );
    assert!(error.to_string().contains("LinearTerm"), "{error}");
}

/// The cosine law needs the using segment's length.
#[test]
fn a_harmonic_spiral_without_a_segment_length_is_refused() {
    let model = spiral("IFCCOSINESPIRAL", &[Some(200.0), None]);
    let session = LoweringSession::new(&model, &METRES);
    assert!(spiral_law(&session, EntityId(1), "IFCCOSINESPIRAL", 0.0).is_err());
}

/// Forward a piece reads `k(start + t)`; backward `-k(start - t)`.
#[test]
fn a_piece_law_is_rebased_and_reversed_in_closed_form() {
    let clothoid = CurvatureLaw::Polynomial {
        coefficients: vec![0.0, 1e-4],
    };
    let forward = piece_law(&clothoid, 50.0, 30.0).expect("forward");
    assert_close(polynomial(&forward), &[0.005, 1e-4]);
    let backward = piece_law(&clothoid, 50.0, -30.0).expect("backward");
    assert_close(polynomial(&backward), &[-0.005, 1e-4]);

    let cosine = CurvatureLaw::Composite {
        polynomial: vec![0.0025, 1e-5],
        harmonics: vec![Harmonic {
            amplitude: 0.005,
            angular_frequency: PI / 50.0,
            phase: FRAC_PI_2,
        }],
    };
    let reversed = piece_law(&cosine, 20.0, -15.0).expect("reversed");
    for t in [0.0, 3.5, 7.0, 15.0] {
        let expected = -evaluate(&cosine, 20.0 - t);
        assert!(
            (evaluate(&reversed, t) - expected).abs() <= 1e-15,
            "t = {t}"
        );
    }
}

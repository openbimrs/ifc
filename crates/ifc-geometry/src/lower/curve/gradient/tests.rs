//! `IfcGradientCurve` read-back against hand-computed stations.
//!
//! The alignment: plan = 100 m of line then a 50 m arc of R = 200 m left;
//! profile = 0.02 grade over 0..60, a parabola to -0.01 over 60..120
//! (`c = -0.03 / 120 = -0.00025`), -0.01 over 120..150. Every station below
//! is closed form: plan points on a line or arc, heights from the
//! polynomials. Arc lengths quoted for the authored `SegmentLength`s are
//! `L sqrt(1 + g^2)` for the lines and the elementary parabola integral
//! (mpmath, 30 digits) for the parabola.

use std::f64::consts::FRAC_PI_2;

use axiolid_curve::{CurvatureLaw, Curve2, Curve3, Elevated3, ElevationLaw};
use axiolid_model::GeometryNode;
use ifc_model::{EntityId, Value};

use super::super::fixture::{Builder, METRES};
use super::super::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::solid::testkit::{entity, r};
use crate::transform::Transform;
use crate::GeometryResult;

/// The authored alignment, with knobs for the refusal cases.
struct Spec {
    arc_origin: [f64; 2],
    arc_direction: [f64; 2],
    parabola_length: f64,
    parabola_direction: [f64; 2],
    vertical_arc: bool,
    profile_end: f64,
    parabola_last: bool,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            arc_origin: [100.0, 0.0],
            arc_direction: [1.0, 0.0],
            parabola_length: 60.002_999_835_023_03,
            parabola_direction: [1.0, 0.02],
            vertical_arc: false,
            profile_end: 150.0,
            parabola_last: false,
        }
    }
}

/// Returns the builder and the gradient curve id.
fn alignment(spec: &Spec) -> (Builder, u64) {
    let mut b = Builder::new();
    let line = b.line();
    let arc = b.circle(200.0);
    let h1 = b.segment_at([0.0, 0.0], [1.0, 0.0], 100.0, line);
    let h2 = b.segment_at(spec.arc_origin, spec.arc_direction, 50.0, arc);
    let closing = b.segment_at(
        [149.480_791_850_904_6, 6.217_515_657_871_043],
        [0.25_f64.cos(), 0.25_f64.sin()],
        0.0,
        line,
    );
    let base = b.composite(&[h1, h2, closing]);

    let first = if spec.vertical_arc {
        b.circle(1000.0)
    } else {
        line
    };
    let v1 = b.segment_at([0.0, 10.0], [1.0, 0.02], 60.011_998_800_239_94, first);
    let parabola = b.parabola(&[11.2, 0.02, -0.000_25]);
    let v2 = b.segment_at(
        [60.0, 11.2],
        spec.parabola_direction,
        spec.parabola_length,
        parabola,
    );
    let v3 = b.segment_at(
        [120.0, 11.5],
        [1.0, -0.01],
        (spec.profile_end - 120.0) * 1.0001_f64.sqrt(),
        line,
    );
    let height = 11.5 - 0.01 * (spec.profile_end - 120.0);
    let v4 = b.segment_at([spec.profile_end, height], [1.0, -0.01], 0.0, line);
    let vertical: Vec<u64> = if spec.parabola_last {
        vec![v1, v2]
    } else {
        vec![v1, v2, v3, v4]
    };
    let gradient = b.add(entity(
        "IFCGRADIENTCURVE",
        vec![
            Value::List(vertical.into_iter().map(r).collect()),
            Value::Bool(false),
            r(base),
            Value::Null,
        ],
    ));
    (b, gradient)
}

fn lower(b: &Builder, id: u64, frame: Transform) -> GeometryResult<Elevated3> {
    let mut session = LoweringSession::new(&b.model, &METRES);
    let root = lower_curve_node(&mut session, EntityId(id), frame)?;
    let lowered = session.finish(root)?;
    match lowered.graph.get(lowered.root) {
        Some(GeometryNode::Curve3(Curve3::Elevated(curve))) => Ok(curve.clone()),
        other => panic!("expected an elevated curve, got {other:?}"),
    }
}

fn at(curve: &Elevated3, d: f64) -> [f64; 3] {
    axiolid_evaluate::elevated_point(curve, d)
        .expect("evaluate")
        .to_array()
}

fn assert_near(actual: [f64; 3], expected: [f64; 3]) {
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() <= 1e-8, "{actual:?} vs {expected:?}");
    }
}

/// The laws themselves: one piecewise plan, one piecewise profile.
#[test]
fn a_gradient_curve_lowers_to_one_plan_and_one_profile() {
    let (b, gradient) = alignment(&Spec::default());
    let curve = lower(&b, gradient, Transform::identity()).expect("gradient curve");
    let Curve2::Intrinsic(plan) = curve.plan.as_ref() else {
        panic!("expected an intrinsic plan");
    };
    assert_eq!(plan.length, 150.0);
    assert_eq!(
        plan.curvature,
        CurvatureLaw::piecewise(
            vec![100.0],
            vec![
                CurvatureLaw::straight(),
                CurvatureLaw::circular(1.0 / 200.0)
            ]
        )
    );
    let ElevationLaw::Piecewise { breaks, laws } = &curve.elevation else {
        panic!("expected a piecewise profile");
    };
    assert_eq!(breaks, &vec![60.0, 120.0]);
    let expected = [
        vec![10.0, 0.02],
        vec![11.2, 0.02, -0.000_25],
        vec![11.5, -0.01],
    ];
    for (law, want) in laws.iter().zip(expected) {
        let ElevationLaw::Polynomial { coefficients } = law else {
            panic!("expected polynomial pieces");
        };
        assert_eq!(coefficients.len(), want.len());
        for (c, w) in coefficients.iter().zip(want) {
            assert!((c - w).abs() <= 1e-15, "{coefficients:?}");
        }
    }
}

/// Stations on the line, the parabola and the arc, all closed form.
#[test]
fn gradient_curve_stations_match_hand_computed_points() {
    let (b, gradient) = alignment(&Spec::default());
    let curve = lower(&b, gradient, Transform::identity()).expect("gradient curve");
    assert_near(at(&curve, 30.0), [30.0, 0.0, 10.6]);
    // Parabola, t = 30: 11.2 + 0.02 * 30 - 0.00025 * 900.
    assert_near(at(&curve, 90.0), [90.0, 0.0, 11.575]);
    // Arc, 25 m in: 100 + 200 sin(0.125), 200 (1 - cos(0.125)).
    assert_near(
        at(&curve, 125.0),
        [124.934_946_677_045_54, 1.560_466_554_134_189, 11.45],
    );
    assert_near(
        at(&curve, 150.0),
        [149.480_791_850_904_6, 6.217_515_657_871_043, 11.2],
    );
}

/// A frame that keeps the vertical moves the plan and lifts the profile.
#[test]
fn a_vertical_keeping_frame_is_applied_exactly() {
    let (b, gradient) = alignment(&Spec::default());
    let frame = Transform {
        basis: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        origin: [1000.0, 2000.0, 50.0],
    };
    let curve = lower(&b, gradient, frame).expect("placed gradient curve");
    assert_near(at(&curve, 30.0), [1000.0, 2030.0, 60.6]);

    let tilted = Transform {
        basis: [
            [1.0, 0.0, 0.0],
            [0.0, FRAC_PI_2.cos(), FRAC_PI_2.sin()],
            [0.0, -FRAC_PI_2.sin(), FRAC_PI_2.cos()],
        ],
        origin: [0.0, 0.0, 0.0],
    };
    let error = lower(&b, gradient, tilted).expect_err("a tilted frame");
    assert!(error.is_unsupported(), "{error}");
}

/// Each inconsistency or missing law is a typed refusal naming its entity.
#[test]
fn gradient_curve_refusals_are_typed() {
    let refused = |spec: Spec, needle: &str| {
        let (b, gradient) = alignment(&spec);
        let error = lower(&b, gradient, Transform::identity()).expect_err(needle);
        assert!(error.is_unsupported(), "{error}");
        assert!(error.to_string().contains(needle), "{needle}: {error}");
    };
    refused(
        Spec {
            vertical_arc: true,
            ..Spec::default()
        },
        "#258",
    );
    refused(
        Spec {
            arc_direction: [0.1_f64.cos(), 0.1_f64.sin()],
            ..Spec::default()
        },
        "kink",
    );
    refused(
        Spec {
            arc_origin: [100.5, 0.0],
            ..Spec::default()
        },
        "gap",
    );
    refused(
        Spec {
            parabola_length: 60.0,
            ..Spec::default()
        },
        "arc length",
    );
    refused(
        Spec {
            parabola_direction: [1.0, 0.0],
            ..Spec::default()
        },
        "start tangent",
    );
    refused(
        Spec {
            parabola_last: true,
            ..Spec::default()
        },
        "#90",
    );
    refused(
        Spec {
            profile_end: 140.0,
            ..Spec::default()
        },
        "must end where the base curve ends",
    );
}

/// Cant has no neutral roll law: always refused, naming #93.
#[test]
fn a_segmented_reference_curve_is_refused_citing_cant() {
    let (mut b, gradient) = alignment(&Spec::default());
    let reference = b.add(entity(
        "IFCSEGMENTEDREFERENCECURVE",
        vec![
            Value::List(vec![]),
            Value::Bool(false),
            r(gradient),
            Value::Null,
        ],
    ));
    let mut session = LoweringSession::new(&b.model, &METRES);
    let error = lower_curve_node(&mut session, EntityId(reference), Transform::identity())
        .expect_err("cant");
    assert!(error.is_unsupported(), "{error}");
    assert_eq!(error.entity(), Some(EntityId(reference)));
    assert!(error.to_string().contains("#93"), "{error}");
}

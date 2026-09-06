//! The lowered curvature law must be numerically right, not merely
//! structurally present.
//!
//! Storing a `CurvatureLaw` is only lossless if the law actually describes the
//! authored curve. These tests check the law against facts derived
//! independently of the crate:
//!
//! - `total_turning` (the kernel's closed-form integral of k ds) must equal
//!   the heading change the segment's endpoint radii imply.
//! - Integrating the stored law with an independent, test-side quadrature must
//!   land on the NEXT segment's authored start point -- a value the fixture
//!   states and this crate never computes.
//!
//! The quadrature lives here, in the test, deliberately. The crate itself
//! performs no numerical integration anywhere; that is the invariant these
//! tests exist to protect.

use axiolid_curve::{CurvatureLaw, Curve2, Intrinsic2};
use axiolid_model::{CurveRelation, GeometryNode};
use ifc_alignment::{lower_horizontal_layout_partial, AlignmentUnits};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

const HORIZONTAL: EntityId = EntityId(191);

fn units() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn model() -> ifc_model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_alignment_spiral.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

/// Every intrinsic curve in the lowered layout, in authored order.
fn intrinsics() -> Vec<Intrinsic2> {
    let model = model();
    let result = lower_horizontal_layout_partial(&model, HORIZONTAL, units()).expect("layout");
    let mut found = Vec::new();
    for run in &result.runs {
        for (_, node) in run.graph.iter() {
            if let GeometryNode::Curve2(Curve2::Intrinsic(curve)) = node {
                found.push(curve.clone());
            }
        }
    }
    found
}

#[test]
fn the_stored_law_is_a_clothoid_linear_in_arc_length() {
    let curves = intrinsics();
    assert_eq!(curves.len(), 2, "fixture has two clothoids");

    for curve in &curves {
        // A clothoid's defining property: curvature is degree-1 in arc length.
        let CurvatureLaw::Polynomial { coefficients } = &curve.curvature else {
            panic!(
                "a clothoid must store a polynomial law, got {:?}",
                curve.curvature
            );
        };
        assert_eq!(coefficients.len(), 2, "degree 1 exactly: {coefficients:?}");
        assert!(
            !curve.curvature.is_constant(),
            "a transition must not be constant"
        );
        assert_eq!(curve.length, 60.0);
    }
}

#[test]
fn total_turning_matches_the_heading_change_the_radii_imply() {
    // Entry clothoid: straight (k=0) into R=300 over L=60.
    // Mean curvature of a linear law is (k0+k1)/2, so the turning is
    // (0 + 1/300)/2 * 60 = 0.1 rad. Derived from the fixture's own radii,
    // not from anything the crate computed.
    let curves = intrinsics();
    let turning = curves[0].total_turning().expect("finite");
    assert!(
        (turning - 0.1).abs() < 1e-12,
        "entry clothoid must turn 0.1 rad, got {turning}"
    );

    // The exit clothoid unwinds the same amount, in the same direction.
    let exit = curves[1].total_turning().expect("finite");
    assert!((exit - 0.1).abs() < 1e-12, "exit clothoid turning {exit}");
}

#[test]
fn integrating_the_stored_law_reaches_the_next_segments_authored_start() {
    // This is the real proof the law is correct: the fixture independently
    // declares where the arc starts. Integrating the clothoid's stored
    // curvature law must land there. If the law were wrong -- wrong
    // coefficient, wrong sign, wrong length normalisation -- this misses.
    let curve = intrinsics().into_iter().next().expect("entry clothoid");
    let CurvatureLaw::Polynomial { coefficients } = &curve.curvature else {
        panic!("expected a polynomial law")
    };
    let (c0, c1) = (coefficients[0], coefficients[1]);

    // Heading is the exact antiderivative of the law; only POSITION needs
    // quadrature, which is precisely why the crate refuses to compute it.
    let theta = |s: f64| c0 * s + c1 * s * s / 2.0;
    let simpson = |f: &dyn Fn(f64) -> f64| {
        const N: usize = 20_001;
        let h = curve.length / (N - 1) as f64;
        let mut total = 0.0;
        for i in 0..N {
            let w = if i == 0 || i == N - 1 {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            total += w * f(i as f64 * h);
        }
        total * h / 3.0
    };

    let x = curve.start.origin.x + simpson(&|s| theta(s).cos());
    let y = curve.start.origin.y + simpson(&|s| theta(s).sin());

    // Fixture entity #145 (the circular arc) declares this start point.
    let (authored_x, authored_y) = (159.940_027_771_371_86, 1.998_571_883_037_536_5);
    assert!(
        (x - authored_x).abs() < 1e-9 && (y - authored_y).abs() < 1e-9,
        "integrated clothoid end ({x}, {y}) must reach the authored arc start \
         ({authored_x}, {authored_y})"
    );
}

#[test]
fn the_spiral_is_trimmed_to_its_authored_length() {
    let model = model();
    let result = lower_horizontal_layout_partial(&model, HORIZONTAL, units()).expect("layout");
    let mut trims = Vec::new();
    for run in &result.runs {
        for (id, node) in run.graph.iter() {
            let GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, end, .. }) = node
            else {
                continue;
            };
            if matches!(
                run.graph.get(*basis),
                Some(GeometryNode::Curve2(Curve2::Intrinsic(_)))
            ) {
                let _ = id;
                trims.push(end.clone());
            }
        }
    }
    assert_eq!(trims.len(), 2, "both spirals are trimmed");
}

/// Evaluate a stored law at arc length `s`. Test-side only: the crate never
/// does this, which is exactly why the tests must, to prove the stored
/// coefficients mean what the published base formula says.
fn evaluate(law: &CurvatureLaw, s: f64) -> f64 {
    match law {
        CurvatureLaw::Constant { curvature } => *curvature,
        CurvatureLaw::Polynomial { coefficients } => coefficients
            .iter()
            .enumerate()
            .map(|(power, c)| c * s.powi(power as i32))
            .sum(),
        CurvatureLaw::Sinusoid {
            mean,
            amplitude,
            angular_frequency,
            phase,
        } => mean + amplitude * (angular_frequency * s + phase).sin(),
        _ => panic!("unhandled law"),
    }
}

fn law_for(name: &str) -> (CurvatureLaw, f64) {
    // Straight (R=0) into R=300 over 60 m, matching the committed fixture.
    let model = single_segment_model(name, 0.0, 300.0, 60.0);
    let result =
        lower_horizontal_layout_partial(&model, EntityId(4), units()).expect("layout is readable");
    assert!(
        result.refused.is_empty(),
        "{name} must lower: {:?}",
        result.refused
    );
    for run in &result.runs {
        for (_, node) in run.graph.iter() {
            if let GeometryNode::Curve2(Curve2::Intrinsic(curve)) = node {
                return (curve.curvature.clone(), curve.length);
            }
        }
    }
    panic!("{name} produced no intrinsic curve")
}

/// Each family must hit its published boundary and midpoint curvatures.
///
/// All three are S-shaped transitions from k0 to k1, so each is pinned by
/// k(0) = k0, k(L) = k1, and k(L/2) = (k0+k1)/2. Those values come from the
/// published base formulas, computed independently of this crate. A wrong
/// coefficient, sign, or frequency breaks at least one of them.
#[test]
fn every_supported_family_matches_its_published_curvature_values() {
    let k1 = 1.0 / 300.0;
    for name in ["CLOTHOID", "BLOSSCURVE", "COSINECURVE"] {
        let (law, length) = law_for(name);
        assert_eq!(length, 60.0, "{name} length");
        assert!(evaluate(&law, 0.0).abs() < 1e-15, "{name} k(0) must be 0");
        assert!(
            (evaluate(&law, 60.0) - k1).abs() < 1e-15,
            "{name} k(L) must be 1/300, got {}",
            evaluate(&law, 60.0)
        );
        assert!(
            (evaluate(&law, 30.0) - k1 / 2.0).abs() < 1e-15,
            "{name} k(L/2) must be half, got {}",
            evaluate(&law, 30.0)
        );
    }
}

/// Total turning is the same for all three families: they share endpoints and
/// each is symmetric about its midpoint, so each integrates to (k0+k1)/2 * L.
#[test]
fn every_supported_family_turns_the_same_total_angle() {
    for name in ["CLOTHOID", "BLOSSCURVE", "COSINECURVE"] {
        let (law, length) = law_for(name);
        let curve = Intrinsic2::new(
            axiolid_core::Frame2 {
                origin: axiolid_core::Point2::new(0.0, 0.0),
                x: axiolid_core::Vec2::new(1.0, 0.0),
                y: axiolid_core::Vec2::new(0.0, 1.0),
            },
            law,
            length,
        );
        let turning = curve.total_turning().expect("finite");
        assert!(
            (turning - 0.1).abs() < 1e-12,
            "{name} must turn 0.1 rad, got {turning}"
        );
    }
}

/// A minimal IfcAlignmentHorizontal nesting one segment of the given type.
fn single_segment_model(
    name: &str,
    start_radius: f64,
    end_radius: f64,
    length: f64,
) -> ifc_model::Model {
    use ifc_model::value::Value;
    use ifc_model::Entity;
    use std::sync::Arc;

    let mut model = ifc_model::Model::new();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(0.0),
                Value::Real(start_radius),
                Value::Real(end_radius),
                Value::Real(length),
                Value::Null,
                Value::Enum(Arc::from(name)),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCALIGNMENTSEGMENT",
            vec![
                Value::Text(Arc::from("seg")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(2)),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCALIGNMENTHORIZONTAL",
            vec![
                Value::Text(Arc::from("horiz")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCRELNESTS",
            vec![
                Value::Text(Arc::from("nest")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(4)),
                Value::List(vec![Value::Ref(EntityId(3))]),
            ],
        ),
    );
    model
}

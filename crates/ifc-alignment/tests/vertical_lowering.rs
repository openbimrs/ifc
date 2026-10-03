//! One vertical segment lowered on its own agrees with the composed profile
//! (#91): both go through `elevation_law`. Circular arcs lower to the circle
//! itself (#258); clothoids state no curvature and stay refused.
//!
//! Heights and grades are hand-computed from the IFC4.3 PARABOLICARC
//! definition (constant rate of change of gradient over plan distance):
//! `z(d) = z0 + g0 d + (g1 - g0) / (2 L) d^2`. For the crest used here,
//! `z0 = 52`, `g0 = +2%`, `g1 = -3%`, `L = 200` starting at station 1100:
//!
//! | local d | station | height | grade  |
//! |--------:|--------:|-------:|-------:|
//! |       0 |    1100 |  52.00 | +0.020 |
//! |      50 |    1150 |  52.6875 | +0.0075 |
//! |     100 |    1200 |  52.75 | -0.005 |
//! |     200 |    1300 |  51.00 | -0.030 |

use std::sync::Arc;

use axiolid_curve::{Curve2, ElevationLaw};
use axiolid_model::{CurveRelation, GeometryNode, TrimSelector};
use ifc_alignment::{
    elevation_law, lower_vertical_segment, profile_law, read_vertical_segment, AlignmentError,
    AlignmentUnits, LoweredAlignmentCurve,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// One `IfcAlignmentVerticalSegment` record, slots 2..8 of IFC4X3_ADD2.
fn record(
    start: f64,
    length: f64,
    height: f64,
    entry: f64,
    exit: f64,
    radius: Option<f64>,
    kind: &str,
) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCALIGNMENTVERTICALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(start),
                Value::Real(length),
                Value::Real(height),
                Value::Real(entry),
                Value::Real(exit),
                radius.map_or(Value::Null, Value::Real),
                Value::Enum(Arc::from(kind)),
            ],
        ),
    );
    model
}

fn crest() -> Model {
    // RadiusOfCurvature L / (g1 - g0) = 200 / -0.05 = -4000.
    record(
        1100.0,
        200.0,
        52.0,
        0.02,
        -0.03,
        Some(-4000.0),
        "PARABOLICARC",
    )
}

/// The trimmed basis curve and its trim end.
fn basis(lowered: &LoweredAlignmentCurve) -> (&Curve2, f64) {
    let Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, end, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a trimmed vertical curve");
    };
    let [TrimSelector::Parameter(end)] = end.as_slice() else {
        panic!("expected a parameter trim");
    };
    let Some(GeometryNode::Curve2(curve)) = lowered.graph.get(*basis) else {
        panic!("expected a Curve2 basis");
    };
    (curve, *end)
}

const TABLE: [(f64, f64, f64); 4] = [
    (0.0, 52.0, 0.02),
    (50.0, 52.6875, 0.0075),
    (100.0, 52.75, -0.005),
    (200.0, 51.0, -0.03),
];

#[test]
fn a_parabolic_arc_lowers_per_segment_to_the_ifc_parabola() {
    let lowered = lower_vertical_segment(&crest(), EntityId(1), metres()).expect("exact parabola");
    let (curve, end) = basis(&lowered);
    assert!(matches!(curve, Curve2::BSpline(b) if b.degree == 2 && b.weights.is_none()));
    assert_eq!(end, 200.0, "trimmed to the horizontal length");

    for (local, height, grade) in TABLE {
        let point = axiolid_evaluate::evaluate2(curve, local).expect("point");
        let derivative = axiolid_evaluate::derivative2(curve, local).expect("derivative");
        assert!(
            (point.x - (1100.0 + local)).abs() < 1e-9,
            "station at {local}"
        );
        assert!(
            (point.y - height).abs() < 1e-12,
            "height at {local}: {} != {height}",
            point.y
        );
        let slope = derivative.y / derivative.x;
        assert!(
            (slope - grade).abs() < 1e-14,
            "grade at {local}: {slope} != {grade}"
        );
    }
}

/// The per-segment curve and the composed law place the same road.
#[test]
fn per_segment_and_composed_paths_agree() {
    let model = crest();
    let law = profile_law(&[read_vertical_segment(&model, EntityId(1), metres()).expect("reads")])
        .expect("composed law");
    let lowered = lower_vertical_segment(&model, EntityId(1), metres()).expect("per segment");
    let (curve, _) = basis(&lowered);
    for local in [0.0, 12.5, 77.0, 150.0, 200.0] {
        let point = axiolid_evaluate::evaluate2(curve, local).expect("point");
        let height = law.height_at(local).expect("height");
        assert!(
            (point.y - height).abs() < 1e-12,
            "paths disagree at {local}: {} vs {height}",
            point.y
        );
    }
}

/// A constant gradient keeps its exact line: origin at the start, direction
/// `(1, grade)`, parameter = plan distance.
#[test]
fn a_constant_gradient_still_lowers_to_a_line() {
    let model = record(1000.0, 100.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT");
    let lowered = lower_vertical_segment(&model, EntityId(1), metres()).expect("line");
    let (curve, end) = basis(&lowered);
    let Curve2::Line(line) = curve else {
        panic!("expected a line, got {curve:?}");
    };
    assert_eq!(line.origin.x, 1000.0);
    assert_eq!(line.origin.y, 50.0);
    assert_eq!(line.direction.y, 0.02);
    assert_eq!(end, 100.0);
    // 50 + 0.02 * 100 = 52 at the segment end.
    let point = axiolid_evaluate::evaluate2(curve, end).expect("end");
    assert!((point.y - 52.0).abs() < 1e-12);
}

/// A vertical `CIRCULARARC` is the circle itself on both paths (#258).
///
/// IFC4X3_ADD2 (`IfcAlignmentVerticalSegmentTypeEnum`): the derivative of
/// the vertical angle with respect to the 3D arc length is constant, `1 /
/// R`, and `RadiusOfCurvature` is positive counter-clockwise (a sag). The
/// crest here, `R = -4000` from +2% at station 1100 and height 52, has its
/// centre `R` along the start tangent's left normal: below the road. Every
/// expected height is that circle's own equation about its centre.
#[test]
fn a_circular_arc_lowers_to_the_circle_on_both_paths() {
    let model = record(
        1100.0,
        200.0,
        52.0,
        0.02,
        -0.030_007,
        Some(-4000.0),
        "CIRCULARARC",
    );
    let segment = read_vertical_segment(&model, EntityId(1), metres()).expect("reads");
    let law = elevation_law(&segment).expect("exact circle");
    assert_eq!(law, ElevationLaw::circular_arc(52.0, 0.02, -4000.0));

    let norm = 0.02_f64.hypot(1.0);
    let centre = (1100.0 + 4000.0 * 0.02 / norm, 52.0 - 4000.0 / norm);
    let circle =
        |station: f64| centre.1 + (4000.0_f64.powi(2) - (station - centre.0).powi(2)).sqrt();
    for local in [0.0, 50.0, 100.0, 150.0, 200.0] {
        let height = law.height_at(local).expect("height");
        let expected = circle(1100.0 + local);
        assert!(
            (height - expected).abs() < 1e-9,
            "at {local}: {height} != {expected}"
        );
    }

    // Per segment: the circle, trimmed by angle from the start to where the
    // plan distance 200 ends.
    let lowered = lower_vertical_segment(&model, EntityId(1), metres()).expect("per segment");
    let (curve, end) = basis(&lowered);
    let Curve2::Circle(arc) = curve else {
        panic!("a vertical arc is a circle, got {curve:?}");
    };
    assert_eq!(arc.radius, 4000.0);
    assert!((arc.frame.origin.x - centre.0).abs() < 1e-9);
    assert!((arc.frame.origin.y - centre.1).abs() < 1e-9);
    let start = axiolid_evaluate::evaluate2(curve, 0.0).expect("start");
    assert!((start.x - 1100.0).abs() < 1e-9 && (start.y - 52.0).abs() < 1e-9);
    let tangent = axiolid_evaluate::derivative2(curve, 0.0).expect("tangent");
    assert!((tangent.y / tangent.x - 0.02).abs() < 1e-12, "start grade");
    let finish = axiolid_evaluate::evaluate2(curve, end).expect("end");
    assert!(
        (finish.x - 1300.0).abs() < 1e-9,
        "ends at the plan length: {}",
        finish.x
    );
    assert!((finish.y - circle(1300.0)).abs() < 1e-9);
    // The two paths place the same road in between.
    for fraction in [0.25, 0.5, 0.75] {
        let point = axiolid_evaluate::evaluate2(curve, end * fraction).expect("point");
        let height = law.height_at(point.x - 1100.0).expect("height");
        assert!(
            (point.y - height).abs() < 1e-9,
            "paths disagree at {}",
            point.x
        );
    }
}

/// A vertical clothoid states neither end curvature, so its law is
/// undetermined: refused identically on both paths, by name.
#[test]
fn a_vertical_clothoid_is_refused_on_both_paths() {
    let model = record(1100.0, 200.0, 52.0, 0.02, -0.03, None, "CLOTHOID");
    let segment = read_vertical_segment(&model, EntityId(1), metres()).expect("reads");
    let composed = elevation_law(&segment).expect_err("no stated curvature");
    let per_segment =
        lower_vertical_segment(&model, EntityId(1), metres()).expect_err("no stated curvature");
    assert_eq!(per_segment, composed, "the paths must agree");
    let AlignmentError::Unsupported {
        entity,
        type_name,
        detail,
    } = per_segment
    else {
        panic!("expected Unsupported, got {per_segment}");
    };
    assert_eq!(entity, EntityId(1));
    assert_eq!(type_name, "CLOTHOID");
    assert!(detail.contains("states neither"), "{detail}");
}

/// A radius turning against the authored grades, and an arc that turns
/// vertical inside its plan length, are invalid data on both paths.
#[test]
fn a_contradictory_circular_arc_is_refused_on_both_paths() {
    for (radius, needle) in [(4000.0, "turns against"), (-100.0, "turns vertical")] {
        let model = record(0.0, 200.0, 52.0, 0.02, -0.03, Some(radius), "CIRCULARARC");
        let segment = read_vertical_segment(&model, EntityId(1), metres()).expect("reads");
        let composed = elevation_law(&segment).expect_err("contradiction");
        assert_eq!(
            lower_vertical_segment(&model, EntityId(1), metres()),
            Err(composed.clone())
        );
        assert!(
            matches!(&composed, AlignmentError::InvalidSegment { detail, .. } if detail.contains(needle)),
            "R {radius}: {composed}"
        );
    }
}

/// The family checks are shared too: a CONSTANTGRADIENT with a radius is
/// invalid data on both paths.
#[test]
fn a_contradictory_constant_gradient_is_refused_on_both_paths() {
    let model = record(0.0, 100.0, 50.0, 0.02, 0.03, None, "CONSTANTGRADIENT");
    let segment = read_vertical_segment(&model, EntityId(1), metres()).expect("reads");
    assert_eq!(
        lower_vertical_segment(&model, EntityId(1), metres()),
        Err(elevation_law(&segment).expect_err("unequal grades"))
    );
}

/// Real exports write a straight grade with `RadiusOfCurvature = 0.`
/// (Trimble Quadri, IFC4.x-IF STN01) and with start and end gradients that
/// differ in the last bit (Trimble, IFC4.x-IF). Both state one straight
/// grade; a non-zero radius or a real grade change is still refused.
#[test]
fn exporter_spellings_of_a_straight_grade_are_read_as_one() {
    let zero_radius = record(
        0.0,
        100.0,
        35.0,
        -0.015,
        -0.015,
        Some(0.0),
        "CONSTANTGRADIENT",
    );
    let lowered = lower_vertical_segment(&zero_radius, EntityId(1), metres()).expect("zero radius");
    let (curve, _) = basis(&lowered);
    // 35 - 0.015 * 100 = 33.5.
    let end = axiolid_evaluate::evaluate2(curve, 100.0).expect("end");
    assert!((end.y - 33.5).abs() < 1e-12, "end height {}", end.y);

    let last_bit = record(
        47.123_889_803_846_9,
        47.123_889_803_846_9,
        -2.0,
        0.042_441_318_157_838_8,
        0.042_441_318_157_838_7,
        None,
        "CONSTANTGRADIENT",
    );
    lower_vertical_segment(&last_bit, EntityId(1), metres()).expect("one grade up to rounding");

    for (radius, exit) in [(Some(1000.0), -0.015), (None, -0.015 + 1e-6)] {
        let model = record(0.0, 100.0, 35.0, -0.015, exit, radius, "CONSTANTGRADIENT");
        assert!(
            lower_vertical_segment(&model, EntityId(1), metres()).is_err(),
            "radius {radius:?}, exit {exit}"
        );
    }
}

use std::sync::Arc;

use axiolid_curve::{Curve2, Line2};
use axiolid_model::{CurveRelation, GeometryNode, TrimSelector};
use ifc_alignment::{lower_horizontal_segment, AlignmentError, AlignmentUnits};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;

fn n(value: f64) -> Value {
    Value::Real(value)
}
fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn e(name: &str) -> Value {
    Value::Enum(Arc::from(name))
}
fn entity(name: &str, attributes: Vec<Value>) -> Entity {
    Entity::new(name, attributes)
}

fn point2(x: f64, y: f64) -> Entity {
    entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(x), n(y)])])
}

fn segment(kind: &str, start_radius: f64, end_radius: f64, length: f64) -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), point2(1_000.0, 2_000.0));
    model.insert(
        EntityId(2),
        entity(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                r(1),
                n(0.0),
                n(start_radius),
                n(end_radius),
                n(length),
                Value::Null,
                e(kind),
            ],
        ),
    );
    model
}

fn millimetres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    }
}

#[test]
fn line_parameters_lower_to_a_finite_neutral_line() {
    let model = segment("LINE", 0.0, 0.0, 5_000.0);
    let lowered = lower_horizontal_segment(&model, EntityId(2), millimetres()).expect("line");

    let GeometryNode::CurveRelation(CurveRelation::Trimmed {
        basis, start, end, ..
    }) = lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("trimmed root")
    };
    assert_eq!(start, &vec![TrimSelector::Parameter(0.0)]);
    assert_eq!(end, &vec![TrimSelector::Parameter(5.0)]);
    let GeometryNode::Curve2(Curve2::Line(Line2 { origin, direction })) =
        lowered.graph.get(*basis).expect("basis")
    else {
        panic!("line basis")
    };
    assert_eq!(*origin, axiolid_core::Point2::new(1.0, 2.0));
    assert_eq!(*direction, axiolid_core::Vec2::X);
}

#[test]
fn circular_arc_keeps_signed_curvature_and_exact_sweep() {
    let model = segment("CIRCULARARC", 10_000.0, 10_000.0, 15_707.963_267_948_966);
    let lowered = lower_horizontal_segment(&model, EntityId(2), millimetres()).expect("arc");

    let GeometryNode::CurveRelation(CurveRelation::Trimmed {
        basis, start, end, ..
    }) = lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("trimmed root")
    };
    assert_eq!(start, &vec![TrimSelector::Parameter(0.0)]);
    let TrimSelector::Parameter(end_angle) = end[0] else {
        panic!("parameter end")
    };
    assert!((end_angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    let GeometryNode::Curve2(Curve2::Circle(circle)) = lowered.graph.get(*basis).expect("basis")
    else {
        panic!("circle basis")
    };
    assert_eq!(circle.radius, 10.0);
    assert_eq!(circle.frame.origin, axiolid_core::Point2::new(1.0, 12.0));
}

#[test]
fn inconsistent_segment_semantics_are_typed_errors() {
    let line = segment("LINE", 1.0, 0.0, 5.0);
    assert!(matches!(
        lower_horizontal_segment(&line, EntityId(2), millimetres()),
        Err(AlignmentError::InvalidSegment { .. })
    ));

    let arc = segment("CIRCULARARC", 10.0, 11.0, 5.0);
    assert!(matches!(
        lower_horizontal_segment(&arc, EntityId(2), millimetres()),
        Err(AlignmentError::InvalidSegment { .. })
    ));
}

#[test]
fn finite_arc_inputs_that_overflow_the_circle_frame_are_rejected() {
    let mut model = segment("CIRCULARARC", 1.7e308, 1.7e308, 1.0);
    model.insert(EntityId(1), point2(1.7e308, 1.7e308));

    assert!(matches!(
        lower_horizontal_segment(
            &model,
            EntityId(2),
            AlignmentUnits {
                length_to_metres: 1.0,
                angle_to_radians: 1.0,
            },
        ),
        Err(AlignmentError::InvalidSegment { .. })
    ));
}

#[test]
fn finite_arc_inputs_that_overflow_the_trim_parameter_are_rejected() {
    let model = segment(
        "CIRCULARARC",
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE,
        f64::MAX,
    );

    assert!(matches!(
        lower_horizontal_segment(
            &model,
            EntityId(2),
            AlignmentUnits {
                length_to_metres: 1.0,
                angle_to_radians: 1.0,
            },
        ),
        Err(AlignmentError::InvalidSegment { .. })
    ));
}

#[test]
fn large_global_coordinates_do_not_collapse_the_arc_frame_axes() {
    let mut model = segment("CIRCULARARC", 1.0, 1.0, 1.0);
    model.insert(EntityId(1), point2(1.0e16, 1.0e16));
    let lowered = lower_horizontal_segment(
        &model,
        EntityId(2),
        AlignmentUnits {
            length_to_metres: 1.0,
            angle_to_radians: 1.0,
        },
    )
    .expect("finite arc");

    let GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, .. }) =
        lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("trimmed root")
    };
    let GeometryNode::Curve2(Curve2::Circle(circle)) = lowered.graph.get(*basis).expect("basis")
    else {
        panic!("circle basis")
    };
    assert_eq!(circle.frame.x, axiolid_core::Vec2::new(0.0, -1.0));
    assert_eq!(circle.frame.y, axiolid_core::Vec2::new(1.0, 0.0));
}

#[test]
fn transition_intent_is_not_approximated() {
    let model = segment("CLOTHOID", 0.0, 10_000.0, 5_000.0);
    assert!(matches!(
        lower_horizontal_segment(&model, EntityId(2), millimetres()),
        Err(AlignmentError::Unsupported { type_name, .. }) if type_name == "CLOTHOID"
    ));
}

#[test]
fn committed_ifc_fixture_lowers_line_and_arc_exactly() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_conic_offset_bounded.ifc");
    let source = std::fs::read_to_string(&path).expect("fixture reads");
    assert!(
        source.contains("FILE_SCHEMA(('IFC4X3_ADD2'))"),
        "alignment entities require an IFC4X3 schema declaration"
    );
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    let line = lower_horizontal_segment(&model, EntityId(61), units).expect("line");
    let arc = lower_horizontal_segment(&model, EntityId(62), units).expect("arc");
    assert!(matches!(
        line.graph.get(line.root),
        Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { .. }))
    ));
    assert!(matches!(
        arc.graph.get(arc.root),
        Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { .. }))
    ));
}

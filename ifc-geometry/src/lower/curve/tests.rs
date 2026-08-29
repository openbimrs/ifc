//! Unit tests for exact curve lowering.
//!
//! The assertion that earns its keep is the trim-parameter one: a conic
//! parameter is an angle and a line parameter is a length, so a single
//! length factor applied to both silently rescales every arc in a
//! millimetre file.

use axiolid_curve::Curve3;
use axiolid_model::{CurveRelation, GeometryNode, Transition, TrimSelector};
use ifc_model::{EntityId, Model, Value};

use super::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::lower::Tolerance;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

/// Millimetre model: lengths scale by 1/1000, angles are already radians.
fn millimetres() -> UnitScale {
    UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    }
}

fn point(x: f64, y: f64, z: f64) -> ifc_model::Entity {
    entity(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![n(x), n(y), n(z)])],
    )
}

/// A circle trimmed by two parameters, in a millimetre model.
fn trimmed_circle() -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    model.insert(
        EntityId(3),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(4),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    model.insert(EntityId(5), entity("IFCCIRCLE", vec![r(4), n(130.5)]));
    model.insert(
        EntityId(6),
        entity(
            "IFCTRIMMEDCURVE",
            vec![
                r(5),
                Value::List(vec![n(0.0)]),
                Value::List(vec![n(0.5)]),
                Value::Bool(true),
                Value::Enum("PARAMETER".into()),
            ],
        ),
    );
    model
}

/// A conic trim parameter is an ANGLE and must not take the length factor.
///
/// With `length_to_metres = 0.001`, scaling 0.5 as a length yields 0.0005 rad
/// -- a 1000x shorter arc that still renders, just wrong.
#[test]
fn a_conic_trim_parameter_is_an_angle_not_a_length() {
    let model = trimmed_circle();
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_curve_node(&mut session, EntityId(6), Transform::identity())
        .expect("the trimmed circle must lower");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::CurveRelation(CurveRelation::Trimmed { start, end, .. }) => {
            assert_eq!(start, &vec![TrimSelector::Parameter(0.0)]);
            assert_eq!(
                end,
                &vec![TrimSelector::Parameter(0.5)],
                "the angle must pass through unscaled, not become 0.0005"
            );
        }
        other => panic!("expected a Trimmed relation, got {other:?}"),
    }
}

/// The circle's own radius IS a length and does scale.
#[test]
fn a_circle_radius_is_converted_to_metres() {
    let model = trimmed_circle();
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_curve_node(&mut session, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Curve3(Curve3::Circle(circle)) => {
            assert!(
                (circle.radius - 0.1305).abs() < 1e-12,
                "130.5 mm must become 0.1305 m, got {}",
                circle.radius
            );
        }
        other => panic!("expected a Circle, got {other:?}"),
    }
}

/// A closed polyline drops the duplicated closing vertex.
///
/// Keeping both the repeat and the `closed` flag yields a zero-length final
/// segment, which a sweep turns into a degenerate frame.
#[test]
fn a_closed_polyline_records_the_flag_instead_of_repeating_the_vertex() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(EntityId(2), point(1000.0, 0.0, 0.0));
    model.insert(EntityId(3), point(1000.0, 1000.0, 0.0));
    model.insert(
        EntityId(4),
        entity(
            "IFCPOLYLINE",
            vec![Value::List(vec![r(1), r(2), r(3), r(1)])],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_curve_node(&mut session, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Curve3(Curve3::Polyline(pl)) => {
            assert!(pl.closed, "the repeated first point means closed");
            assert_eq!(pl.points.len(), 3, "the duplicate must not be stored");
            assert!((pl.points[1].x - 1.0).abs() < 1e-12, "1000 mm -> 1 m");
        }
        other => panic!("expected a Polyline, got {other:?}"),
    }
}

/// Composite segment order and per-segment sense are preserved verbatim.
///
/// Reordering or dropping a segment changes the swept path; a sweep along it
/// still produces a solid, just the wrong one.
#[test]
fn composite_segments_keep_their_order_and_sense() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(EntityId(2), point(1000.0, 0.0, 0.0));
    model.insert(EntityId(3), point(2000.0, 0.0, 0.0));
    model.insert(
        EntityId(4),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(2), r(3)])]),
    );
    model.insert(
        EntityId(6),
        entity(
            "IFCCOMPOSITECURVESEGMENT",
            vec![Value::Enum("CONTINUOUS".into()), Value::Bool(true), r(4)],
        ),
    );
    model.insert(
        EntityId(7),
        entity(
            "IFCCOMPOSITECURVESEGMENT",
            vec![
                Value::Enum("DISCONTINUOUS".into()),
                Value::Bool(false),
                r(5),
            ],
        ),
    );
    model.insert(
        EntityId(8),
        entity(
            "IFCCOMPOSITECURVE",
            vec![Value::List(vec![r(6), r(7)]), Value::Bool(false)],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_curve_node(&mut session, EntityId(8), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::CurveRelation(CurveRelation::Composite { segments }) => {
            assert_eq!(segments.len(), 2, "both segments must survive");
            assert!(segments[0].same_sense, "first segment is forward");
            assert!(!segments[1].same_sense, "second segment is reversed");
            assert_eq!(segments[0].transition, Transition::Continuous);
            assert_eq!(segments[1].transition, Transition::Discontinuous);
        }
        other => panic!("expected a Composite relation, got {other:?}"),
    }
}

/// An unknown curve family is a typed report, never a substituted shape.
#[test]
fn an_unsupported_curve_family_is_reported_by_name() {
    let mut model = Model::new();
    model.insert(EntityId(1), entity("IFCBSPLINECURVEWITHKNOTS", vec![]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_curve_node(&mut session, EntityId(1), Transform::identity())
        .expect_err("an unlowered family must not silently succeed");
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert_eq!(error.entity(), Some(EntityId(1)));
}

/// An `IfcVector`'s magnitude is part of the line's parameterisation.
///
/// `IfcLine` is `origin + t * direction` where `direction` is an `IfcVector`
/// carrying its own `Magnitude`. Normalizing it away rescales `t`, so every
/// trim taken on the line then selects a different piece of curve. The
/// geometry still evaluates -- it is simply the wrong length.
#[test]
fn a_line_direction_keeps_the_vector_magnitude() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    // Magnitude 2000 mm -> 2 m; the stored direction must be (2,0,0), not
    // the unit vector.
    model.insert(EntityId(3), entity("IFCVECTOR", vec![r(2), n(2000.0)]));
    model.insert(EntityId(4), entity("IFCLINE", vec![r(1), r(3)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_curve_node(&mut session, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Curve3(Curve3::Line(line)) => {
            let d = line.direction.to_array();
            assert!(
                (d[0] - 2.0).abs() < 1e-12,
                "magnitude 2000 mm must survive as 2 m, got {d:?}"
            );
        }
        other => panic!("expected a Line, got {other:?}"),
    }
}

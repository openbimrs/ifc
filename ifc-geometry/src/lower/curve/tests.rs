//! Unit tests for exact curve lowering.
//!
//! The assertion that earns its keep is the trim-parameter one: a conic
//! parameter is an angle and a line parameter is a length, so a single
//! length factor applied to both silently rescales every arc in a
//! millimetre file.

use axiolid_curve::{Curve2, Curve3};
use axiolid_model::{CurveRelation, GeometryNode, MasterRepresentation, Transition, TrimSelector};
use ifc_model::{EntityId, Model, Value};

use super::{lower_curve_node, scale_parameter};
use crate::lower::session::LoweringSession;
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
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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

#[test]
fn ellipse_axes_are_lowered_exactly_in_source_orientation() {
    let mut model = trimmed_circle();
    model.insert(
        EntityId(7),
        entity("IFCELLIPSE", vec![r(4), n(250.0), n(125.0)]),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_curve_node(&mut session, EntityId(7), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Curve3(Curve3::Ellipse(ellipse)) => {
            assert_eq!(ellipse.semi_axis_x, 0.25);
            assert_eq!(ellipse.semi_axis_y, 0.125);
            assert_eq!(ellipse.frame.x.to_array(), [1.0, 0.0, 0.0]);
            assert_eq!(ellipse.frame.z.to_array(), [0.0, 0.0, 1.0]);
        }
        other => panic!("expected an Ellipse, got {other:?}"),
    }
}

#[test]
fn offset_curve_preserves_basis_distance_and_reference_direction() {
    let mut model = trimmed_circle();
    model.insert(
        EntityId(7),
        entity(
            "IFCOFFSETCURVE3D",
            vec![r(5), n(50.0), Value::Bool(false), r(3)],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_curve_node(&mut session, EntityId(7), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::CurveRelation(CurveRelation::Offset {
            basis,
            distance,
            reference_direction,
        }) => {
            assert_eq!(*distance, 0.05);
            assert_eq!(
                reference_direction.expect("3D offset direction").to_array(),
                [1.0, 0.0, 0.0]
            );
            assert!(matches!(
                lowered.graph.get(*basis),
                Some(GeometryNode::Curve3(Curve3::Circle(_)))
            ));
        }
        other => panic!("expected an Offset relation, got {other:?}"),
    }
}

/// A still-unlowered curve family is a typed report, never a substituted shape.
#[test]
fn an_unsupported_curve_family_is_reported_by_name() {
    let mut model = Model::new();
    model.insert(EntityId(1), entity("IFCPOLYNOMIALCURVE", vec![]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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

#[test]
fn parameter_units_follow_curve_parameterisation() {
    let model = Model::new();
    let scale = millimetres();
    let session = LoweringSession::new(&model, &scale);

    assert_eq!(scale_parameter(&session, "IFCLINE", 2_000.0), 2_000.0);
    assert_eq!(scale_parameter(&session, "IFCPOLYLINE", 2.0), 2.0);
    assert_eq!(scale_parameter(&session, "IFCCOMPOSITECURVE", 2_000.0), 2.0);
    assert_eq!(scale_parameter(&session, "IFCCIRCLE", 0.5), 0.5);
}

#[test]
fn surface_curve_keeps_master_and_raw_parameter_coordinates() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(0.0), n(0.0)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(1.5), n(2.0)])]),
    );
    model.insert(
        EntityId(6),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(4), r(5)])]),
    );
    model.insert(EntityId(7), entity("IFCPCURVE", vec![r(3), r(6)]));
    model.insert(
        EntityId(8),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(EntityId(9), entity("IFCVECTOR", vec![r(8), n(1000.0)]));
    model.insert(EntityId(10), entity("IFCLINE", vec![r(1), r(9)]));
    model.insert(
        EntityId(11),
        entity(
            "IFCSURFACECURVE",
            vec![
                r(10),
                Value::List(vec![r(7)]),
                Value::Enum("CURVE3D".into()),
            ],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(11), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
        curve_3d,
        sides,
        master,
    }) = lowered.graph.get(root).expect("root")
    else {
        panic!("expected surface curve relation");
    };
    assert_eq!(*master, MasterRepresentation::Curve3d);
    assert!(matches!(
        lowered.graph.get(*curve_3d),
        Some(GeometryNode::Curve3(Curve3::Line(_)))
    ));
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(sides.first().1).expect("pcurve")
    else {
        panic!("expected parameter curve relation");
    };
    let Some(GeometryNode::Curve2(Curve2::Polyline(polyline))) =
        lowered.graph.get(*reference_curve)
    else {
        panic!("expected parameter-space polyline");
    };
    assert_eq!(
        polyline.points[1].to_array(),
        [1.5, 2.0],
        "surface parameters must not use project length units"
    );
}

#[test]
fn indexed_polycurve_lowers_line_and_three_point_arc_segments() {
    let mut model = Model::new();
    let coords = [
        [0.0, 0.0, 0.0],
        [1000.0, 0.0, 0.0],
        [1000.0, 1000.0, 0.0],
        [0.0, 1000.0, 0.0],
    ];
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(
                coords
                    .into_iter()
                    .map(|p| Value::List(p.into_iter().map(n).collect()))
                    .collect(),
            )],
        ),
    );
    let index = |name: &str, values: &[i64]| Value::Typed {
        type_name: name.into(),
        value: Box::new(Value::List(
            values.iter().copied().map(Value::Integer).collect(),
        )),
    };
    model.insert(
        EntityId(2),
        entity(
            "IFCINDEXEDPOLYCURVE",
            vec![
                r(1),
                Value::List(vec![
                    index("IFCLINEINDEX", &[1, 2]),
                    index("IFCARCINDEX", &[2, 3, 4]),
                ]),
                Value::Bool(false),
            ],
        ),
    );
    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(2), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::Composite { segments }) =
        lowered.graph.get(root).expect("root")
    else {
        panic!("expected composite indexed curve");
    };
    assert_eq!(segments.len(), 2);
    assert!(matches!(
        lowered.graph.get(segments[0].curve),
        Some(GeometryNode::Curve3(Curve3::Polyline(_)))
    ));
    let GeometryNode::CurveRelation(CurveRelation::Trimmed {
        basis,
        start,
        end,
        sense_agreement,
        ..
    }) = lowered.graph.get(segments[1].curve).expect("arc")
    else {
        panic!("expected trimmed circular arc");
    };
    let Some(GeometryNode::Curve3(Curve3::Circle(circle))) = lowered.graph.get(*basis) else {
        panic!("expected circle basis");
    };
    assert!((circle.radius - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert!(matches!(start.as_slice(), [TrimSelector::Point3(_)]));
    assert!(matches!(end.as_slice(), [TrimSelector::Point3(_)]));
    assert!(
        *sense_agreement,
        "the chosen middle point lies on the positive sweep"
    );
}

#[test]
fn indexed_arc_rejects_finite_points_when_derived_circle_values_overflow() {
    let mut model = Model::new();
    let coords = [[0.0, 0.0, 0.0], [1.0e200, 0.0, 0.0], [0.0, 1.0e-200, 0.0]];
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(
                coords
                    .into_iter()
                    .map(|point| Value::List(point.into_iter().map(n).collect()))
                    .collect(),
            )],
        ),
    );
    let arc_index = Value::Typed {
        type_name: "IFCARCINDEX".into(),
        value: Box::new(Value::List(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ])),
    };
    model.insert(
        EntityId(2),
        entity(
            "IFCINDEXEDPOLYCURVE",
            vec![r(1), Value::List(vec![arc_index]), Value::Bool(false)],
        ),
    );
    let scale = UnitScale {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale);

    assert!(matches!(
        lower_curve_node(&mut session, EntityId(2), Transform::identity()),
        Err(crate::error::GeometryError::Degenerate { .. })
    ));
}

/// `PCURVE_S2` names the second parametric side and now lowers.
///
/// This was refused outright: the neutral master could not distinguish S1
/// from S2, so picking either would have been a guess. With each side
/// pairing a surface to its own p-curve, the master names the second side
/// exactly.
#[test]
fn surface_curve_master_can_name_the_second_pcurve() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(0.0), n(0.0)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(1.5), n(2.0)])]),
    );
    model.insert(
        EntityId(6),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(4), r(5)])]),
    );
    model.insert(EntityId(7), entity("IFCPCURVE", vec![r(3), r(6)]));
    model.insert(EntityId(12), entity("IFCPCURVE", vec![r(3), r(6)]));
    model.insert(
        EntityId(8),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(EntityId(9), entity("IFCVECTOR", vec![r(8), n(1000.0)]));
    model.insert(EntityId(10), entity("IFCLINE", vec![r(1), r(9)]));
    model.insert(
        EntityId(11),
        entity(
            "IFCSURFACECURVE",
            vec![
                r(10),
                Value::List(vec![r(7), r(12)]),
                Value::Enum("PCURVE_S2".into()),
            ],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(11), Transform::identity())
        .expect("PCURVE_S2 with two p-curves lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::SurfaceCurve { sides, master, .. }) =
        lowered.graph.get(root).expect("root")
    else {
        panic!("expected surface curve relation");
    };
    assert_eq!(*master, MasterRepresentation::ParameterCurveS2);
    assert!(sides.is_two_sided(), "both parametric sides are recorded");
}

/// `PCURVE_S2` with only one p-curve names a side that does not exist.
#[test]
fn surface_curve_master_naming_a_missing_side_is_refused() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(0.0), n(0.0)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(1.5), n(2.0)])]),
    );
    model.insert(
        EntityId(6),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(4), r(5)])]),
    );
    model.insert(EntityId(7), entity("IFCPCURVE", vec![r(3), r(6)]));
    model.insert(
        EntityId(8),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(EntityId(9), entity("IFCVECTOR", vec![r(8), n(1000.0)]));
    model.insert(EntityId(10), entity("IFCLINE", vec![r(1), r(9)]));
    model.insert(
        EntityId(11),
        entity(
            "IFCSURFACECURVE",
            vec![
                r(10),
                Value::List(vec![r(7)]),
                Value::Enum("PCURVE_S2".into()),
            ],
        ),
    );

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_curve_node(&mut session, EntityId(11), Transform::identity())
        .expect_err("PCURVE_S2 with one p-curve is inconsistent");
    let text = format!("{error:?}");
    assert!(
        text.contains("PCURVE_S2"),
        "the refusal must name the inconsistency, got: {text}"
    );
}

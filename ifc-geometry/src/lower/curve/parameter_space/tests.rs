//! Unit tests for parameter-space (p-curve) reference curve lowering.
//!
//! Every model here is authored in **millimetres**. That is the point: a
//! surface parameter must never receive the project length factor, so if any
//! of these values were scaled they would come back a thousand times smaller
//! and the assertions would fail. The unit choice is the assertion.

use axiolid_curve::Curve2;
use axiolid_model::{CurveRelation, GeometryNode};
use ifc_model::{EntityId, Model, Value};

use crate::lower::curve::lower_curve_node;
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

/// A 3D cartesian point.
fn point(x: f64, y: f64, z: f64) -> ifc_model::Entity {
    entity(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![n(x), n(y), n(z)])],
    )
}

/// `IfcIndexedPolyCurve` with no explicit `Segments` reads as a plain
/// dimensionless parameter-space point sequence, exactly like the
/// `IfcPolyline` case above. Proves the p-curve reference-curve family is
/// no longer limited to `IfcPolyline`.
#[test]
fn p_curve_accepts_an_implicit_indexed_polycurve_reference() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity(
            "IFCCARTESIANPOINTLIST2D",
            vec![Value::List(vec![
                Value::List(vec![n(0.0), n(0.0)]),
                Value::List(vec![n(1.5), n(2.0)]),
            ])],
        ),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCINDEXEDPOLYCURVE",
            vec![r(4), Value::Null, Value::Bool(false)],
        ),
    );
    model.insert(EntityId(6), entity("IFCPCURVE", vec![r(3), r(5)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(6), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(root).expect("root")
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

/// An `IfcIndexedPolyCurve` with an explicit arc segment cannot be read as a
/// plain point sequence, and this crate carries no parameter-space arc
/// contract yet, so it must stay a named typed refusal rather than silently
/// flattening the arc to a straight line.
#[test]
fn p_curve_refuses_an_indexed_polycurve_with_an_explicit_arc_segment() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity(
            "IFCCARTESIANPOINTLIST2D",
            vec![Value::List(vec![
                Value::List(vec![n(0.0), n(0.0)]),
                Value::List(vec![n(1.0), n(1.0)]),
                Value::List(vec![n(2.0), n(0.0)]),
            ])],
        ),
    );
    let index = |name: &str, values: &[i64]| Value::Typed {
        type_name: name.into(),
        value: Box::new(Value::List(
            values.iter().copied().map(Value::Integer).collect(),
        )),
    };
    model.insert(
        EntityId(5),
        entity(
            "IFCINDEXEDPOLYCURVE",
            vec![
                r(4),
                Value::List(vec![index("IFCARCINDEX", &[1, 2, 3])]),
                Value::Bool(false),
            ],
        ),
    );
    model.insert(EntityId(6), entity("IFCPCURVE", vec![r(3), r(5)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_curve_node(&mut session, EntityId(6), Transform::identity())
        .expect_err("explicit-segment parameter-space indexed polycurves are not yet represented");
    assert!(error.is_unsupported());
    assert_eq!(error.entity(), Some(EntityId(5)));
}

/// A parameter-space `IfcCircle` keeps its authored radius verbatim. The
/// model is in millimetres, so a radius that received the project length
/// factor would come back as 0.0015 rather than 1.5. A p-curve circle's
/// "radius" is a displacement in the surface's own (u, v) domain, which for a
/// cylinder mixes an angle with a length, so no length factor may apply.
#[test]
fn p_curve_circle_keeps_its_radius_in_parameter_space() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(2.0), n(3.0)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCAXIS2PLACEMENT2D", vec![r(4), Value::Null]),
    );
    model.insert(EntityId(6), entity("IFCCIRCLE", vec![r(5), n(1.5)]));
    model.insert(EntityId(7), entity("IFCPCURVE", vec![r(3), r(6)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(7), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(root).expect("root")
    else {
        panic!("expected parameter curve relation");
    };
    let Some(GeometryNode::Curve2(Curve2::Circle(circle))) = lowered.graph.get(*reference_curve)
    else {
        panic!("expected parameter-space circle");
    };
    assert_eq!(
        circle.radius, 1.5,
        "surface parameters must not use project length units"
    );
    assert_eq!(circle.frame.origin.to_array(), [2.0, 3.0]);
    assert_eq!(circle.frame.x.to_array(), [1.0, 0.0]);
    assert_eq!(
        circle.frame.y.to_array(),
        [0.0, 1.0],
        "local Y is the orthogonal complement of X"
    );
}

/// A rotated `RefDirection` must reach the kernel frame verbatim, and the
/// derived Y must be X rotated a quarter turn counter-clockwise
/// (`IfcOrthogonalComplement`). Deriving Y the other way would mirror every
/// parameter-space conic.
#[test]
fn p_curve_conic_frame_follows_the_authored_ref_direction() {
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
    // RefDirection = +Y, i.e. the frame is rotated a quarter turn.
    model.insert(
        EntityId(5),
        entity("IFCDIRECTION", vec![Value::List(vec![n(0.0), n(1.0)])]),
    );
    model.insert(EntityId(6), entity("IFCAXIS2PLACEMENT2D", vec![r(4), r(5)]));
    model.insert(EntityId(7), entity("IFCCIRCLE", vec![r(6), n(2.0)]));
    model.insert(EntityId(8), entity("IFCPCURVE", vec![r(3), r(7)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(8), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(root).expect("root")
    else {
        panic!("expected parameter curve relation");
    };
    let Some(GeometryNode::Curve2(Curve2::Circle(circle))) = lowered.graph.get(*reference_curve)
    else {
        panic!("expected parameter-space circle");
    };
    assert_eq!(circle.frame.x.to_array(), [0.0, 1.0]);
    assert_eq!(
        circle.frame.y.to_array(),
        [-1.0, 0.0],
        "Y must be X rotated counter-clockwise, not clockwise"
    );
}

/// A parameter-space `IfcEllipse` keeps both semi-axes verbatim, in
/// declaration order, mapped to the frame's local X and Y.
#[test]
fn p_curve_ellipse_keeps_both_semi_axes_in_parameter_space() {
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
        entity("IFCAXIS2PLACEMENT2D", vec![r(4), Value::Null]),
    );
    model.insert(
        EntityId(6),
        entity("IFCELLIPSE", vec![r(5), n(3.0), n(0.5)]),
    );
    model.insert(EntityId(7), entity("IFCPCURVE", vec![r(3), r(6)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(7), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(root).expect("root")
    else {
        panic!("expected parameter curve relation");
    };
    let Some(GeometryNode::Curve2(Curve2::Ellipse(ellipse))) = lowered.graph.get(*reference_curve)
    else {
        panic!("expected parameter-space ellipse");
    };
    assert_eq!(
        (ellipse.semi_axis_x, ellipse.semi_axis_y),
        (3.0, 0.5),
        "surface parameters must not use project length units"
    );
}

/// A parameter-space `IfcLine` keeps its `Dir` magnitude: `Dir` is an
/// `IfcVector`, so its length sets the parameter scale. Normalizing it, or
/// applying the project length factor to it, would reparameterise the line.
#[test]
fn p_curve_line_keeps_its_direction_magnitude_unscaled() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(1.0), n(2.0)])]),
    );
    model.insert(
        EntityId(5),
        entity("IFCDIRECTION", vec![Value::List(vec![n(1.0), n(0.0)])]),
    );
    // Magnitude 4.0 must survive verbatim, not become 0.004 under millimetres.
    model.insert(EntityId(6), entity("IFCVECTOR", vec![r(5), n(4.0)]));
    model.insert(EntityId(7), entity("IFCLINE", vec![r(4), r(6)]));
    model.insert(EntityId(8), entity("IFCPCURVE", vec![r(3), r(7)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_curve_node(&mut session, EntityId(8), Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("finishes");
    let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
        basis_surface: _,
        reference_curve,
    }) = lowered.graph.get(root).expect("root")
    else {
        panic!("expected parameter curve relation");
    };
    let Some(GeometryNode::Curve2(Curve2::Line(line))) = lowered.graph.get(*reference_curve) else {
        panic!("expected parameter-space line");
    };
    assert_eq!(line.origin.to_array(), [1.0, 2.0]);
    assert_eq!(
        line.direction.to_array(),
        [4.0, 0.0],
        "Dir magnitude sets the parameter scale and must be preserved unscaled"
    );
}

/// A parameter-space conic positioned by an `IfcAxis2Placement3D` is a typed
/// refusal: a 3D placement carries an axis that has no meaning in a surface's
/// two-dimensional (u, v) domain, so silently dropping its Z would invent a
/// projection the file never authored.
#[test]
fn p_curve_conic_refuses_a_three_dimensional_placement() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(EntityId(4), entity("IFCCIRCLE", vec![r(2), n(1.0)]));
    model.insert(EntityId(5), entity("IFCPCURVE", vec![r(3), r(4)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_curve_node(&mut session, EntityId(5), Transform::identity())
        .expect_err("a 3D placement has no meaning in a parameter domain");
    assert!(error.is_unsupported());
    assert_eq!(error.entity(), Some(EntityId(2)));
}

/// B-splines stay a named typed refusal: their knot vector would need an
/// explicit parameter-domain contract this crate does not yet carry.
#[test]
fn p_curve_still_refuses_a_parameter_space_bspline() {
    let mut model = Model::new();
    model.insert(EntityId(1), point(0.0, 0.0, 0.0));
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));
    model.insert(
        EntityId(4),
        entity("IFCBSPLINECURVEWITHKNOTS", vec![Value::Integer(3)]),
    );
    model.insert(EntityId(5), entity("IFCPCURVE", vec![r(3), r(4)]));

    let scale = millimetres();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_curve_node(&mut session, EntityId(5), Transform::identity())
        .expect_err("parameter-space B-splines are not yet represented");
    assert!(error.is_unsupported());
    assert_eq!(error.entity(), Some(EntityId(4)));
}

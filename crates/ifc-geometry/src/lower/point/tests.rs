use super::*;
use crate::units::UnitScale;
use axiolid_curve::Curve3;
use ifc_model::{Entity, Model, Value};

fn model_with_polyline_and_circle() -> (Model, EntityId, EntityId) {
    let mut model = Model::new();
    // #1..#3: three IfcCartesianPoint for the polyline.
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(10.0),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(10.0),
                Value::Real(10.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCPOLYLINE",
            vec![Value::List(vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(2)),
                Value::Ref(EntityId(3)),
            ])],
        ),
    );
    // #5: IfcPointOnCurve referencing the polyline at parameter 1.0 (segment ordinal).
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCPOINTONCURVE",
            vec![Value::Ref(EntityId(4)), Value::Real(1.0)],
        ),
    );
    // #6: circle centre point + placement, #7 circle, #8 point-on-curve on circle at angle (radians).
    model.insert(
        EntityId(6),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(7),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(6)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(8),
        Entity::new("IFCCIRCLE", vec![Value::Ref(EntityId(7)), Value::Real(5.0)]),
    );
    (model, EntityId(4), EntityId(8))
}

fn scale() -> UnitScale {
    UnitScale::default()
}

fn session<'a>(model: &'a Model, scale: &'a UnitScale) -> LoweringSession<'a> {
    LoweringSession::new(model, scale)
}

#[test]
fn point_on_curve_preserves_basis_and_scaled_parameter() {
    let (model, polyline, _circle) = model_with_polyline_and_circle();
    let unit_scale = scale();
    let mut sess = session(&model, &unit_scale);
    let node =
        lower_point_on_curve_node(&mut sess, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = sess.finish(node).expect("finish");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::PointOnCurve(p) => {
            assert_eq!(
                p.parameter, 1.0,
                "polyline parameter is a dimensionless ordinal, not length-scaled"
            );
            let basis_node = lowered.graph.get(p.curve).expect("basis curve node");
            assert!(matches!(
                basis_node,
                GeometryNode::Curve3(Curve3::Polyline(_))
            ));
        }
        other => panic!("expected PointOnCurve, got {other:?}"),
    }
    let _ = polyline;
}

#[test]
fn point_on_curve_on_a_circle_converts_the_parameter_as_an_angle() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCCIRCLE", vec![Value::Ref(EntityId(2)), Value::Real(5.0)]),
    );
    // Parameter in degrees (unit scale below converts degrees->radians via angle()).
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCPOINTONCURVE",
            vec![Value::Ref(EntityId(3)), Value::Real(90.0)],
        ),
    );

    let units = UnitScale {
        length_to_metres: 1.0,
        angle_to_radians: std::f64::consts::PI / 180.0,
    };
    let mut sess = LoweringSession::new(&model, &units);
    let node =
        lower_point_on_curve_node(&mut sess, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = sess.finish(node).expect("finish");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::PointOnCurve(p) => {
            assert!(
                (p.parameter - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
                "got {}",
                p.parameter
            );
        }
        other => panic!("expected PointOnCurve, got {other:?}"),
    }
}

#[test]
fn point_on_surface_preserves_basis_and_both_parameters() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCPLANE", vec![Value::Ref(EntityId(2))]),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCPOINTONSURFACE",
            vec![Value::Ref(EntityId(3)), Value::Real(2.0), Value::Real(3.0)],
        ),
    );
    let scale = UnitScale::default();
    let mut sess = LoweringSession::new(&model, &scale);
    let node =
        lower_point_on_surface_node(&mut sess, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = sess.finish(node).expect("finish");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::PointOnSurface(p) => {
            assert_eq!((p.u, p.v), (2.0, 3.0));
        }
        other => panic!("expected PointOnSurface, got {other:?}"),
    }
}

#[test]
fn a_missing_basis_curve_reports_the_dangling_reference() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCPOINTONCURVE",
            vec![Value::Ref(EntityId(99)), Value::Real(1.0)],
        ),
    );
    let scale = UnitScale::default();
    let mut sess = LoweringSession::new(&model, &scale);
    let err = lower_point_on_curve_node(&mut sess, EntityId(1), Transform::identity()).unwrap_err();
    assert_eq!(err.entity(), Some(EntityId(99)));
}

//! Unit tests for the trim-unit rules of [`super`].

use std::f64::consts::FRAC_PI_2;

use axiolid_model::{GeometryNode, SurfaceRelation};
use ifc_model::{EntityId, Model, Value};

use crate::error::GeometryError;
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

/// A STEP list of reals, the shape coordinates and ratios take.
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().map(|v| n(*v)).collect())
}

/// A trim parameter on a revolved basis is an ANGLE, not a length.
///
/// This is the assertion that pays for the whole family. With degrees in the
/// file and a length factor applied, a 90-degree patch becomes 0.09 -- still a
/// valid surface, just the wrong one.
#[test]
fn a_trim_parameter_on_a_curved_basis_uses_the_angle_unit() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        entity("IFCCYLINDRICALSURFACE", vec![r(2), Value::Real(200.0)]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                r(3),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(90.0),
                Value::Real(500.0),
                Value::Bool(true),
                Value::Bool(true),
            ],
        ),
    );

    // Degrees for angle, millimetres for length: the two factors differ by
    // orders of magnitude, so a swapped factor cannot pass by luck.
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: Some(0.017453292519943295),
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_surface_node(&mut session, EntityId(4), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::RectangularTrimmed { u, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a rectangular trim");
    };
    let expected = 90.0 * 0.017453292519943295;
    assert!(
        (u.1 - expected).abs() < 1e-12,
        "u2 must be radians ({expected}), got {} -- a length factor gives 0.09",
        u.1
    );
}

/// A trim's `v` on a cylinder is a length, its `u` an angle (#589).
///
/// ISO 10303-42 parameterises a cylinder as `(r cos u, r sin u, v)`. One
/// factor for both directions leaves `v` in millimetres in a file whose other
/// lengths are metres.
#[test]
fn a_cylinder_trim_scales_u_as_an_angle_and_v_as_a_length() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        entity("IFCCYLINDRICALSURFACE", vec![r(2), Value::Real(200.0)]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                r(3),
                Value::Real(0.0),
                Value::Real(100.0),
                Value::Real(90.0),
                Value::Real(500.0),
                Value::Bool(true),
                Value::Bool(true),
            ],
        ),
    );
    assert_trim(&model, EntityId(4), (0.0, FRAC_PI_2), (0.1, 0.5));
}

/// A revolved surface's `v` is its generatrix's own parameter (#589).
///
/// `u` is the revolution angle. `v` is not a length: on a polyline it is a
/// point ordinal and stays as written, on a circle it is an angle.
fn revolution_trim_model(circle_generatrix: bool) -> Model {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[200.0, 0.0, 0.0])]),
    );
    if circle_generatrix {
        model.insert(
            EntityId(2),
            entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
        );
        model.insert(
            EntityId(3),
            entity("IFCCIRCLE", vec![r(2), Value::Real(50.0)]),
        );
    } else {
        model.insert(
            EntityId(2),
            entity("IFCCARTESIANPOINT", vec![reals(&[200.0, 0.0, 300.0])]),
        );
        model.insert(
            EntityId(3),
            entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
        );
    }
    model.insert(
        EntityId(4),
        entity(
            "IFCARBITRARYOPENPROFILEDEF",
            vec![Value::Enum("CURVE".into()), Value::Null, r(3)],
        ),
    );
    model.insert(
        EntityId(5),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(6),
        entity("IFCDIRECTION", vec![reals(&[0.0, 0.0, 1.0])]),
    );
    model.insert(EntityId(7), entity("IFCAXIS1PLACEMENT", vec![r(5), r(6)]));
    model.insert(
        EntityId(8),
        entity("IFCSURFACEOFREVOLUTION", vec![r(4), Value::Null, r(7)]),
    );
    model.insert(
        EntityId(9),
        entity(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                r(8),
                Value::Real(0.0),
                Value::Real(0.25),
                Value::Real(180.0),
                Value::Real(1.0),
                Value::Bool(true),
                Value::Bool(true),
            ],
        ),
    );
    model
}

#[test]
fn a_revolution_trim_on_a_polyline_generatrix_leaves_v_unscaled() {
    let model = revolution_trim_model(false);
    assert_trim(
        &model,
        EntityId(9),
        (0.0, std::f64::consts::PI),
        (0.25, 1.0),
    );
}

#[test]
fn a_revolution_trim_on_a_circle_generatrix_takes_v_as_an_angle() {
    let model = revolution_trim_model(true);
    let rad = 0.017453292519943295;
    assert_trim(
        &model,
        EntityId(9),
        (0.0, std::f64::consts::PI),
        (0.25 * rad, rad),
    );
}

/// A basis whose parameter units are not modelled is refused, not defaulted.
#[test]
fn a_trim_on_a_linear_extrusion_basis_is_unsupported() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCCARTESIANPOINT", vec![reals(&[1.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(4),
        entity("IFCDIRECTION", vec![reals(&[0.0, 0.0, 1.0])]),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCSURFACEOFLINEAREXTRUSION",
            vec![r(3), Value::Null, r(4), Value::Real(5.0)],
        ),
    );
    model.insert(
        EntityId(6),
        entity(
            "IFCRECTANGULARTRIMMEDSURFACE",
            vec![
                r(5),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(1.0),
                Value::Real(1.0),
                Value::Bool(true),
                Value::Bool(true),
            ],
        ),
    );
    let scale = UnitScale::SI;
    let mut session = LoweringSession::new(&model, &scale);
    let err = lower_surface_node(&mut session, EntityId(6), Transform::identity())
        .expect_err("an unmodelled parameter unit must be refused");
    assert!(
        matches!(&err, GeometryError::Unsupported { entity, type_name, .. }
            if *entity == EntityId(6) && type_name == "IFCSURFACEOFLINEAREXTRUSION"),
        "got {err:?}"
    );
}

/// `IfcPointOnSurface` converts `u` and `v` with the same per-direction rule.
#[test]
fn a_point_on_a_cylinder_scales_u_as_an_angle_and_v_as_a_length() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        entity("IFCCYLINDRICALSURFACE", vec![r(2), Value::Real(200.0)]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCPOINTONSURFACE",
            vec![r(3), Value::Real(90.0), Value::Real(500.0)],
        ),
    );
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: Some(0.017453292519943295),
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node = crate::lower::point::lower_point_on_surface_node(
        &mut session,
        EntityId(4),
        Transform::identity(),
    )
    .expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::PointOnSurface(point)) = lowered.graph.get(lowered.root) else {
        panic!("expected a point on a surface");
    };
    assert!(
        (point.u - FRAC_PI_2).abs() < 1e-12,
        "u is radians, got {}",
        point.u
    );
    assert!(
        (point.v - 0.5).abs() < 1e-12,
        "v is metres, got {}",
        point.v
    );
}

/// Lower a trimmed surface in a millimetre-and-degree file and compare the
/// converted parameter ranges.
fn assert_trim(model: &Model, id: EntityId, u: (f64, f64), v: (f64, f64)) {
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: Some(0.017453292519943295),
    };
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_surface_node(&mut session, id, Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::RectangularTrimmed {
        u: got_u,
        v: got_v,
        ..
    })) = lowered.graph.get(lowered.root)
    else {
        panic!("expected a rectangular trim");
    };
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert!(
        close(got_u.0, u.0) && close(got_u.1, u.1),
        "u must be radians {u:?}, got {got_u:?}"
    );
    assert!(
        close(got_v.0, v.0) && close(got_v.1, v.1),
        "v must be {v:?}, got {got_v:?}"
    );
}

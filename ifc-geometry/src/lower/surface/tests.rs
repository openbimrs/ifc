//! Unit tests for surface lowering.
//!
//! The assertions that earn their keep here are the ones a render would not
//! reveal: a plane that keeps its normal but loses its U/V axes still draws
//! correctly and reparameterises every trim on it, and an extrusion depth
//! folded into the direction changes the surface parameterisation without
//! moving a single pixel.

use axiolid_model::{GeometryNode, SurfaceRelation};
use axiolid_surface::Surface;
use ifc_model::{EntityId, Model, Value};

use super::{lower_linear_extrusion, lower_plane, lower_surface_node};
use crate::lower::session::LoweringSession;
use crate::lower::tolerance::Tolerance;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

/// A plane at `origin` whose placement carries explicit Z and X axes.
fn plane_model(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Model {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(origin.iter().map(|v| n(*v)).collect())],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            "IFCDIRECTION",
            vec![Value::List(axis.iter().map(|v| n(*v)).collect())],
        ),
    );
    model.insert(
        EntityId(3),
        entity(
            "IFCDIRECTION",
            vec![Value::List(ref_dir.iter().map(|v| n(*v)).collect())],
        ),
    );
    model.insert(
        EntityId(4),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    model.insert(EntityId(5), entity("IFCPLANE", vec![r(4)]));
    model
}

fn lower_plane_surface(model: &Model, scale: &UnitScale, frame: Transform) -> Surface {
    let mut session = LoweringSession::new(model, scale, Tolerance::building_scale());
    let node = lower_plane(&mut session, EntityId(5), frame).expect("the plane must lower");
    let lowered = session.finish(node).expect("session finishes");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::Surface(surface) => surface.clone(),
        other => panic!("expected a Surface, got {other:?}"),
    }
}

/// The plane keeps the placement's own U/V axes, not axes derived from Z.
///
/// A plane's `x`/`y` fix its parameterisation. Rebuilding them from the normal
/// picks an arbitrary rotation about it: the plane still renders in exactly
/// the same place, but every `IfcRectangularTrimmedSurface` and pcurve taken
/// against it lands somewhere else.
#[test]
fn a_plane_keeps_the_placement_axes_that_fix_its_parameterisation() {
    // Z = +Y (world), explicit X = +Z (world). A normal-only lowering cannot
    // recover this X: it would derive some other perpendicular.
    let model = plane_model([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
    let surface = lower_plane_surface(&model, &UnitScale::default(), Transform::identity());
    let Surface::Plane(plane) = surface else {
        panic!("expected a plane");
    };
    assert_eq!(
        plane.frame.z.to_array(),
        [0.0, 1.0, 0.0],
        "Z is the placement axis"
    );
    assert_eq!(
        plane.frame.x.to_array(),
        [0.0, 0.0, 1.0],
        "X must be the authored RefDirection, not one derived from Z"
    );
}

/// The plane origin is converted to metres; the axes are not.
///
/// Axes are directions. Scaling them by the length factor leaves them
/// non-unit, and a millimetre file would hand the kernel axes of length 1000.
#[test]
fn a_plane_origin_is_scaled_to_metres_but_its_axes_stay_unit() {
    let model = plane_model([1000.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let surface = lower_plane_surface(&model, &scale, Transform::identity());
    let Surface::Plane(plane) = surface else {
        panic!("expected a plane");
    };
    assert_eq!(
        plane.frame.origin.to_array(),
        [1.0, 0.0, 0.0],
        "1000 mm is 1 m"
    );
    let x = plane.frame.x.to_array();
    let length = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-12,
        "axes must stay unit length, got {length}"
    );
}

/// A non-unit authored axis is normalized into the frame.
///
/// `IfcDirection` is not required to be unit length, and a frame whose axes
/// are not unit is not a frame: parameter distances along it scale by the
/// axis magnitude. This uses a deliberately long axis so the normalization is
/// actually reachable -- a unit input would leave the division dead.
#[test]
fn a_non_unit_authored_axis_is_normalized_into_the_frame() {
    let model = plane_model([0.0, 0.0, 0.0], [0.0, 0.0, 7.0], [4.0, 0.0, 0.0]);
    let surface = lower_plane_surface(&model, &UnitScale::default(), Transform::identity());
    let Surface::Plane(plane) = surface else {
        panic!("expected a plane");
    };
    for (name, axis) in [
        ("x", plane.frame.x.to_array()),
        ("z", plane.frame.z.to_array()),
    ] {
        let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        assert!(
            (length - 1.0).abs() < 1e-12,
            "{name} must be normalized, got length {length}"
        );
    }
}

/// A curved surface reports a typed gap naming the family.
///
/// The readers for these exist; the lowering does not. Reporting the family
/// keeps the gap auditable instead of silently flattening a cylinder to its
/// tangent plane.
#[test]
fn a_curved_surface_is_reported_as_unsupported_by_name() {
    let mut model = plane_model([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    model.insert(
        EntityId(6),
        entity("IFCCYLINDRICALSURFACE", vec![r(4), n(2.5)]),
    );
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_surface_node(&mut session, EntityId(6), Transform::identity())
        .expect_err("cylindrical surfaces are not lowered yet");
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error.to_string().contains("IFCCYLINDRICALSURFACE"),
        "the report must name the family, got: {error}"
    );
}

/// A polyline swept along +Z becomes a linear extrusion of the lowered curve.
#[test]
fn a_linear_extrusion_references_its_swept_curve_and_direction() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    // SweptCurve, Position ($), ExtrudedDirection, Depth
    model.insert(
        EntityId(5),
        entity(
            "IFCSURFACEOFLINEAREXTRUSION",
            vec![r(3), Value::Null, r(4), n(5.0)],
        ),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_linear_extrusion(&mut session, EntityId(5), Transform::identity())
        .expect("the extrusion must lower");
    let lowered = session.finish(node).expect("finishes");

    let relation = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::SurfaceRelation(relation) => relation.clone(),
        other => panic!("expected a SurfaceRelation, got {other:?}"),
    };
    let SurfaceRelation::LinearExtrusion {
        swept_curve,
        direction,
    } = relation
    else {
        panic!("expected a linear extrusion");
    };
    assert_eq!(
        direction.to_array(),
        [0.0, 0.0, 1.0],
        "the extruded direction is carried as a unit direction"
    );
    assert!(
        matches!(
            lowered.graph.get(swept_curve).expect("swept curve node"),
            GeometryNode::Curve3(_)
        ),
        "the swept curve must be a real lowered curve node"
    );
}

/// `Depth` never scales the direction.
///
/// The surface is unbounded in the extrusion parameter; `Depth` is a drawing
/// hint. Folding it into the direction multiplies the parameterisation by the
/// depth, so a point at `v` moves to `v * depth` and every trim against this
/// surface silently shifts. Nothing about the rendered shape reveals it.
#[test]
fn the_depth_hint_never_scales_the_extrusion_direction() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCSURFACEOFLINEAREXTRUSION",
            vec![r(3), Value::Null, r(4), n(1000.0)],
        ),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_linear_extrusion(&mut session, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let GeometryNode::SurfaceRelation(SurfaceRelation::LinearExtrusion { direction, .. }) =
        lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("expected a linear extrusion");
    };
    let d = direction.to_array();
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-12,
        "a Depth of 1000 must not scale the direction, got length {length}"
    );
}

/// A placed extrusion rotates its direction without translating it.
///
/// The direction is a direction: under a frame with a translation it must
/// pick up the rotation only. Running it through the full affine adds the
/// origin offset, so an extrusion authored along +Z tilts once the surface
/// sits away from the world origin -- and the further out it sits, the worse
/// the tilt, which is why an origin-local test cannot see it.
#[test]
fn a_placed_extrusion_rotates_its_direction_but_never_translates_it() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(3),
        entity("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
    );
    model.insert(
        EntityId(4),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCSURFACEOFLINEAREXTRUSION",
            vec![r(3), Value::Null, r(4), n(2.0)],
        ),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let frame = Transform::translation([100.0, -50.0, 25.0]);
    let node = lower_linear_extrusion(&mut session, EntityId(5), frame).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let GeometryNode::SurfaceRelation(SurfaceRelation::LinearExtrusion { direction, .. }) =
        lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("expected a linear extrusion");
    };
    assert_eq!(
        direction.to_array(),
        [0.0, 0.0, 1.0],
        "a pure translation must leave the direction untouched"
    );
}

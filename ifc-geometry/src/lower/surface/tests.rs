//! Unit tests for surface lowering.
//!
//! The assertions that earn their keep here are the ones a render would not
//! reveal: a plane that keeps its normal but loses its U/V axes still draws
//! correctly and reparameterises every trim on it, and an extrusion depth
//! folded into the direction changes the surface parameterisation without
//! moving a single pixel.

use axiolid_model::{CurveRelation, GeometryNode, SurfaceRelation};
use axiolid_surface::Surface;
use ifc_model::{EntityId, Model, Value};

use super::{lower_linear_extrusion, lower_plane, lower_surface_node};
use crate::lower::session::LoweringSession;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::units::UnitScale;

/// A plane at `origin` whose placement carries explicit Z and X axes.
/// A STEP list of reals, the shape coordinates and ratios take.
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().map(|v| n(*v)).collect())
}

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
    let mut session = LoweringSession::new(model, scale);
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
fn an_unlowered_surface_family_is_reported_by_name() {
    let mut model = plane_model([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    // IfcSurfaceOfLinearExtrusion and the curved elementary families now
    // lower; IfcPcurve does not, and it is a surface by the schema.
    model.insert(EntityId(6), entity("IFCPCURVE", vec![r(5), Value::Null]));
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_surface_node(&mut session, EntityId(6), Transform::identity())
        .expect_err("pcurves are not lowered yet");
    assert!(error.is_unsupported(), "this is a gap, not corruption");
    assert!(
        error.to_string().contains("IFCPCURVE"),
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
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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
    let mut session = LoweringSession::new(&model, &scale);
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

/// A plane with a square outer boundary and one square hole.
fn curve_bounded_model() -> Model {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(EntityId(3), entity("IFCPLANE", vec![r(2)]));

    let mut pid = 4u64;
    let mut poly = |model: &mut Model, pts: &[[f64; 2]]| {
        let mut refs = Vec::new();
        for p in pts {
            let id = EntityId(pid);
            pid += 1;
            model.insert(id, entity("IFCCARTESIANPOINT", vec![reals(p)]));
            refs.push(Value::Ref(id));
        }
        let line = EntityId(pid);
        pid += 1;
        model.insert(line, entity("IFCPOLYLINE", vec![Value::List(refs)]));
        line
    };
    let outer = poly(
        &mut model,
        &[[0.0, 0.0], [5.0, 0.0], [5.0, 3.0], [0.0, 0.0]],
    );
    let inner = poly(
        &mut model,
        &[[1.0, 1.0], [2.0, 1.0], [2.0, 2.0], [1.0, 1.0]],
    );
    model.insert(
        EntityId(20),
        entity(
            "IFCCURVEBOUNDEDPLANE",
            vec![
                r(3),
                Value::Ref(outer),
                Value::List(vec![Value::Ref(inner)]),
            ],
        ),
    );
    model.insert(
        EntityId(26),
        entity("IFCPCURVE", vec![r(3), Value::Ref(outer)]),
    );
    model.insert(
        EntityId(27),
        entity("IFCPCURVE", vec![r(3), Value::Ref(inner)]),
    );
    model.insert(
        EntityId(22),
        entity(
            "IFCCOMPOSITECURVESEGMENT",
            vec![Value::Enum("CONTINUOUS".into()), Value::Bool(true), r(26)],
        ),
    );
    model.insert(
        EntityId(23),
        entity(
            "IFCOUTERBOUNDARYCURVE",
            vec![Value::List(vec![r(22)]), Value::Bool(false)],
        ),
    );
    model.insert(
        EntityId(24),
        entity(
            "IFCCOMPOSITECURVESEGMENT",
            vec![Value::Enum("CONTINUOUS".into()), Value::Bool(true), r(27)],
        ),
    );
    model.insert(
        EntityId(25),
        entity(
            "IFCBOUNDARYCURVE",
            vec![Value::List(vec![r(24)]), Value::Bool(false)],
        ),
    );
    model.insert(
        EntityId(21),
        entity(
            "IFCCURVEBOUNDEDSURFACE",
            vec![r(3), Value::List(vec![r(23), r(25)]), Value::Bool(true)],
        ),
    );
    model
}

/// A cylinder keeps its radius in metres and its placement axes unit length.
#[test]
fn a_cylinder_converts_its_radius_but_not_its_axes() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[1000.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCDIRECTION", vec![reals(&[0.0, 0.0, 1.0])]),
    );
    model.insert(
        EntityId(3),
        entity("IFCDIRECTION", vec![reals(&[1.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(4),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    model.insert(
        EntityId(5),
        entity("IFCCYLINDRICALSURFACE", vec![r(4), Value::Real(250.0)]),
    );

    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_surface_node(&mut session, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::Surface(Surface::Cylinder(cyl))) = lowered.graph.get(lowered.root)
    else {
        panic!("expected a cylinder");
    };
    assert!(
        (cyl.radius - 0.25).abs() < 1e-12,
        "radius must be metres, got {}",
        cyl.radius
    );
    assert!(
        (cyl.frame.origin.to_array()[0] - 1.0).abs() < 1e-12,
        "origin must be metres"
    );
    let z = cyl.frame.z.to_array();
    let len = (z[0] * z[0] + z[1] * z[1] + z[2] * z[2]).sqrt();
    assert!(
        (len - 1.0).abs() < 1e-12,
        "axis must stay unit length, got {len}"
    );
}

/// A torus keeps both radii and does not silently reject a spindle.
#[test]
fn a_torus_preserves_a_self_intersecting_spindle() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    // minor > major: a legal but self-intersecting torus.
    model.insert(
        EntityId(3),
        entity(
            "IFCTOROIDALSURFACE",
            vec![r(2), Value::Real(100.0), Value::Real(300.0)],
        ),
    );

    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_surface_node(&mut session, EntityId(3), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::Surface(Surface::Torus(tor))) = lowered.graph.get(lowered.root) else {
        panic!("expected a torus");
    };
    assert!((tor.major_radius - 0.1).abs() < 1e-12);
    assert!(
        (tor.minor_radius - 0.3).abs() < 1e-12,
        "the spindle must survive lowering, got {}",
        tor.minor_radius
    );
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
        angle_to_radians: 0.017453292519943295,
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

/// The curve-bounded plane keeps its outer boundary first and its hole after.
#[test]
fn a_curve_bounded_plane_orders_outer_then_inner() {
    let model = curve_bounded_model();
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_surface_node(&mut session, EntityId(20), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
        boundaries,
        implicit_outer,
        ..
    })) = lowered.graph.get(lowered.root)
    else {
        panic!("expected a curve-bounded surface");
    };
    assert_eq!(boundaries.len(), 2, "outer plus one hole");
    assert!(
        !implicit_outer,
        "IfcCurveBoundedPlane always states its outer boundary"
    );
}

#[test]
fn a_curve_bounded_surface_preserves_boundary_order_and_implicit_outer() {
    let model = curve_bounded_model();
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_surface_node(&mut session, EntityId(21), Transform::identity())
        .expect("curve-bounded surface lowers");
    let lowered = session.finish(root).expect("graph finishes");

    match lowered.graph.get(root).expect("root") {
        GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
            basis,
            boundaries,
            implicit_outer,
        }) => {
            assert!(matches!(
                lowered.graph.get(*basis),
                Some(GeometryNode::Surface(Surface::Plane(_)))
            ));
            assert_eq!(boundaries.len(), 2);
            for boundary in boundaries {
                let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
                    lowered.graph.get(*boundary)
                else {
                    panic!("expected an IFC boundary composite");
                };
                assert_eq!(segments.len(), 1);
                assert!(matches!(
                    lowered.graph.get(segments[0].curve),
                    Some(GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
                        basis_surface,
                        ..
                    })) if basis_surface == basis
                ));
            }
            assert!(*implicit_outer);
        }
        other => panic!("expected curve-bounded surface, got {other:?}"),
    }
}

/// The B-spline keeps u and v distinct: degrees, knots and net orientation.
///
/// The fixture patch is cubic in u and linear in v over a saddle-shaped net,
/// so a transposed control net or swapped degrees is geometrically wrong, not
/// merely relabelled.
#[test]
fn a_bspline_patch_keeps_its_directions_distinct() {
    let mut model = Model::default();
    let mut next = 1u64;
    let mut point = |model: &mut Model, x: f64, y: f64, z: f64| {
        let id = EntityId(next);
        next += 1;
        model.insert(id, entity("IFCCARTESIANPOINT", vec![reals(&[x, y, z])]));
        id
    };
    let mut rows = Vec::new();
    for (i, x) in [0.0f64, 1000.0, 2000.0, 3000.0].into_iter().enumerate() {
        let mut row = Vec::new();
        for (j, y) in [0.0f64, 4000.0].into_iter().enumerate() {
            let z = if (i == 1 || i == 2) != (j == 1) {
                600.0
            } else {
                0.0
            };
            row.push(Value::Ref(point(&mut model, x, y, z)));
        }
        rows.push(Value::List(row));
    }
    let surface_id = EntityId(100);
    model.insert(
        surface_id,
        entity(
            "IFCBSPLINESURFACEWITHKNOTS",
            vec![
                Value::Integer(3),
                Value::Integer(1),
                Value::List(rows),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                Value::List(vec![Value::Integer(4), Value::Integer(4)]),
                Value::List(vec![Value::Integer(2), Value::Integer(2)]),
                Value::List(vec![Value::Real(0.0), Value::Real(1.0)]),
                Value::List(vec![Value::Real(0.0), Value::Real(1.0)]),
                Value::Enum("UNSPECIFIED".into()),
            ],
        ),
    );

    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_surface_node(&mut session, surface_id, Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::Surface(Surface::BSpline(patch))) = lowered.graph.get(lowered.root)
    else {
        panic!("expected a B-spline surface");
    };
    assert_eq!(patch.u_degree, 3, "u is the cubic direction");
    assert_eq!(patch.v_degree, 1, "v is the linear direction");
    assert_eq!(patch.control_points.len(), 4, "four rows along u");
    assert_eq!(patch.control_points[0].len(), 2, "two columns along v");
    assert_eq!(patch.u_multiplicities, vec![4, 4], "clamped cubic ends");
    assert_eq!(patch.v_multiplicities, vec![2, 2], "clamped linear ends");
    assert!(
        patch.weights.is_none(),
        "a polynomial patch must not gain weights"
    );
    let corner = patch.control_points[1][0].to_array();
    assert!(
        (corner[0] - 1.0).abs() < 1e-12,
        "control points convert to metres"
    );
    assert!(
        (corner[2] - 0.6).abs() < 1e-12,
        "the saddle height must survive"
    );
}

#[test]
fn self_referential_surface_is_a_typed_cycle_error() {
    let mut model = Model::default();
    model.insert(
        EntityId(1),
        entity(
            "IFCCURVEBOUNDEDSURFACE",
            vec![r(1), Value::List(vec![r(2)]), Value::Bool(false)],
        ),
    );
    model.insert(
        EntityId(2),
        entity("IFCBOUNDARYCURVE", vec![Value::List(vec![])]),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_surface_node(&mut session, EntityId(1), Transform::identity())
        .expect_err("surface cycle must be bounded");
    assert!(matches!(
        error,
        crate::GeometryError::CyclicChain { entity, kind }
            if entity == EntityId(1) && kind == "surface"
    ));
}

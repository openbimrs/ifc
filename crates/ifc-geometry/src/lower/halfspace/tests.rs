//! Unit tests for half-space lowering.
//!
//! The assertion that earns its keep is the polarity one: IFC's flag and the
//! kernel's flag are opposites, and getting it wrong produces a boolean that
//! still evaluates and still looks like geometry.

use axiolid_curve::Curve2;
use axiolid_model::{GeometryNode, SolidOperation};
use ifc_model::{Entity, EntityId, Model, Value};

use super::lower_half_space_node;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;
use crate::units::UnitScale;

fn entity(type_name: &str, attributes: Vec<Value>) -> Entity {
    Entity::new(type_name, attributes)
}

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn n(v: f64) -> Value {
    Value::Real(v)
}

fn point2(x: f64, y: f64) -> Entity {
    entity("IFCCARTESIANPOINT", vec![Value::List(vec![n(x), n(y)])])
}

fn point3(x: f64, y: f64, z: f64) -> Entity {
    entity(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![n(x), n(y), n(z)])],
    )
}

/// A half space whose base plane sits at `z_offset` with +Z normal.
///
/// `#1` point, `#2` direction (axis), `#3` placement, `#4` plane, `#5` solid.
fn half_space(agreement: bool, z_offset: f64) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(0.0), n(0.0), n(z_offset)])],
        ),
    );
    model.insert(
        EntityId(2),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    model.insert(
        EntityId(3),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), Value::Null]),
    );
    model.insert(EntityId(4), entity("IFCPLANE", vec![r(3)]));
    model.insert(
        EntityId(5),
        entity("IFCHALFSPACESOLID", vec![r(4), Value::Bool(agreement)]),
    );
    model
}

fn lower(model: &Model, frame: Transform) -> axiolid_primitive::HalfSpace {
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(model, &scale);
    let node =
        lower_half_space_node(&mut session, EntityId(5), frame).expect("the half space must lower");
    let lowered = session.finish(node).expect("session finishes");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::HalfSpace(hs) => *hs,
        other => panic!("expected a HalfSpace node, got {other:?}"),
    }
}

/// IFC `.T.` means "away from the normal"; the kernel's `true` means "the
/// normal side". They must come out inverted.
///
/// This is the test that catches a straight-through transcription. Without it
/// a clipping tool keeps the half it was supposed to remove, and every
/// downstream check still passes.
#[test]
fn the_agreement_flag_is_inverted_for_the_kernel() {
    let solid = lower(&half_space(true, 0.0), Transform::identity());
    assert!(
        !solid.agreement,
        "IFC .T. (away from normal) must become kernel false (opposite side)"
    );

    let solid = lower(&half_space(false, 0.0), Transform::identity());
    assert!(
        solid.agreement,
        "IFC .F. must become kernel true (normal side)"
    );
}

/// The plane's placement origin and axis become the boundary plane.
#[test]
fn the_base_placement_becomes_the_boundary_plane() {
    let solid = lower(&half_space(true, 2.5), Transform::identity());
    assert_eq!(solid.boundary.origin.to_array(), [0.0, 0.0, 2.5]);
    assert_eq!(solid.boundary.normal.to_array(), [0.0, 0.0, 1.0]);
}

/// A world frame moves the plane's origin and rotates its normal.
///
/// The normal must take the linear part only. Running it through the full
/// affine adds the translation and tilts every cut by an amount that grows
/// with distance from the origin.
#[test]
fn the_frame_moves_the_origin_but_only_rotates_the_normal() {
    let frame = Transform::translation([10.0, 4.0, 0.0]);
    let solid = lower(&half_space(true, 2.5), frame);

    assert_eq!(
        solid.boundary.origin.to_array(),
        [10.0, 4.0, 2.5],
        "the plane origin is translated"
    );
    assert_eq!(
        solid.boundary.normal.to_array(),
        [0.0, 0.0, 1.0],
        "a pure translation must leave the normal untouched"
    );
}

/// The stored normal is unit length even under a scaling world frame.
///
/// `axis_placement_transform` already normalizes the placement's own axis, so
/// a non-unit `IfcDirection` alone cannot exercise this. A scaled world frame
/// can: composing it stretches the basis, and the resulting normal is only
/// unit length because this module renormalizes after the transform. Without
/// a scaling frame the renormalization is dead code and the test proves
/// nothing -- which is exactly what a surviving mutant revealed.
#[test]
fn the_boundary_normal_is_normalized_under_a_scaling_frame() {
    let model = half_space(true, 0.0);
    let scaled = Transform::identity().scaled(4.0);
    let solid = lower(&model, scaled);

    let normal = solid.boundary.normal.to_array();
    let length = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-12,
        "normal must be unit length after a 4x frame, got {length}"
    );
    assert_eq!(
        normal,
        [0.0, 0.0, 1.0],
        "uniform scale must not change the direction"
    );
}

#[test]
fn non_uniform_frames_transform_plane_normals_as_covectors() {
    let mut model = half_space(true, 0.0);
    model.insert(
        EntityId(2),
        entity(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(1.0), n(0.0)])],
        ),
    );
    let frame = Transform {
        basis: [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        origin: [0.0; 3],
    };
    let normal = lower(&model, frame).boundary.normal.to_array();
    let root_five = 5.0_f64.sqrt();
    assert!((normal[0] - 1.0 / root_five).abs() < 1e-12);
    assert!((normal[1] - 2.0 / root_five).abs() < 1e-12);
    assert!(normal[2].abs() < 1e-12);
}

/// Lengths are converted to metres.
#[test]
fn the_plane_origin_is_converted_to_metres() {
    let model = half_space(true, 2500.0);
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_half_space_node(&mut session, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let solid = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::HalfSpace(hs) => *hs,
        other => panic!("expected HalfSpace, got {other:?}"),
    };
    assert_eq!(
        solid.boundary.origin.to_array(),
        [0.0, 0.0, 2.5],
        "2500 mm must become 2.5 m"
    );
}

/// A polygonal bounded half-space keeps its positioned 2D boundary.
///
/// `Position` is independent of `BaseSurface`, so it must reach the neutral
/// node intact. Lowering this as the unbounded supertype would remove material
/// outside the authored prism; folding `Position` into the clip plane would put
/// the prism in the wrong place.
#[test]
fn a_polygonal_bound_is_never_dropped() {
    let mut model = half_space(true, 0.0);
    // A unit square boundary in Position's XY plane.
    model.insert(EntityId(10), point2(0.0, 0.0));
    model.insert(EntityId(11), point2(1.0, 0.0));
    model.insert(EntityId(12), point2(1.0, 1.0));
    model.insert(EntityId(13), point2(0.0, 1.0));
    model.insert(
        EntityId(6),
        entity(
            "IFCPOLYLINE",
            vec![Value::List(vec![r(10), r(11), r(12), r(13)])],
        ),
    );
    // Position offset from the base surface, which must survive.
    model.insert(EntityId(8), point3(3.0, 4.0, 5.0));
    model.insert(
        EntityId(7),
        entity("IFCAXIS2PLACEMENT3D", vec![r(8), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(4), Value::Bool(true), r(7), r(6)],
        ),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_half_space_node(&mut session, EntityId(5), Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace {
        half_space,
        boundary,
        placement,
    }) = lowered.graph.get(lowered.root).expect("root")
    else {
        panic!("expected a BoundedHalfSpace operation");
    };
    assert_eq!(
        placement.translation.to_array(),
        [3.0, 4.0, 5.0],
        "Position is independent of BaseSurface and must not be dropped"
    );
    assert!(matches!(
        lowered.graph.get(*half_space),
        Some(GeometryNode::HalfSpace(_))
    ));
    // Curve2, not Curve3: the kernel compiler refuses anything else (#45).
    let Some(GeometryNode::Curve2(Curve2::Polyline(boundary_curve))) = lowered.graph.get(*boundary)
    else {
        panic!("expected a 2D polyline boundary");
    };
    assert_eq!(
        boundary_curve.points.len(),
        4,
        "the authored square must keep all four corners"
    );
    assert_eq!(
        boundary_curve.points[2].to_array(),
        [1.0, 1.0],
        "boundary stays in the placement's own XY plane"
    );
}

/// The square from [`a_polygonal_bound_is_never_dropped`] with its boundary
/// points replaced, so each test varies only the boundary.
fn bounded_with_boundary(points: [Entity; 4]) -> Model {
    let mut model = half_space(true, 0.0);
    for (offset, point) in points.into_iter().enumerate() {
        model.insert(EntityId(10 + offset as u64), point);
    }
    model.insert(
        EntityId(6),
        entity(
            "IFCPOLYLINE",
            vec![Value::List(vec![r(10), r(11), r(12), r(13), r(10)])],
        ),
    );
    model.insert(EntityId(8), point3(0.0, 0.0, 0.0));
    model.insert(
        EntityId(7),
        entity("IFCAXIS2PLACEMENT3D", vec![r(8), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(5),
        entity(
            "IFCPOLYGONALBOUNDEDHALFSPACE",
            vec![r(4), Value::Bool(true), r(7), r(6)],
        ),
    );
    model
}

fn lower_boundary(model: &Model, scale: &UnitScale) -> Result<Vec<[f64; 2]>, String> {
    let mut session = LoweringSession::new(model, scale);
    let node = lower_half_space_node(&mut session, EntityId(5), Transform::identity())
        .map_err(|error| format!("{error:?}"))?;
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace { boundary, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a BoundedHalfSpace operation");
    };
    let Some(GeometryNode::Curve2(Curve2::Polyline(curve))) = lowered.graph.get(*boundary) else {
        panic!("expected a Curve2 polyline boundary");
    };
    assert!(curve.closed, "a repeated first point closes the boundary");
    Ok(curve.points.iter().map(|p| p.to_array()).collect())
}

/// Boundary coordinates are lengths, so the project factor applies, and the
/// closing duplicate is carried by `closed` rather than a repeated vertex.
#[test]
fn boundary_points_convert_to_metres_and_drop_the_closing_duplicate() {
    let model = bounded_with_boundary([
        point2(0.0, 0.0),
        point2(2000.0, 0.0),
        point2(2000.0, 1000.0),
        point2(0.0, 1000.0),
    ]);
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let points = lower_boundary(&model, &scale).expect("lowers");
    assert_eq!(
        points,
        vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
        "2000 mm must become 2 m, and the closing point must not repeat"
    );
}

/// Exporters sometimes write 3D points with an explicit zero; that is still
/// the boundary plane and lowers unchanged.
#[test]
fn a_boundary_point_with_zero_z_is_accepted() {
    let model = bounded_with_boundary([
        point3(0.0, 0.0, 0.0),
        point3(1.0, 0.0, 0.0),
        point3(1.0, 1.0, 0.0),
        point3(0.0, 1.0, 0.0),
    ]);
    let points = lower_boundary(&model, &UnitScale::default()).expect("lowers");
    assert_eq!(points[2], [1.0, 1.0]);
}

/// A vertex off Position's XY plane violates `BoundaryDim`. Projecting it
/// would silently move the clip, so it is refused by entity and by name.
#[test]
fn a_boundary_point_off_the_plane_is_refused_by_name() {
    let model = bounded_with_boundary([
        point3(0.0, 0.0, 0.0),
        point3(1.0, 0.0, 0.0),
        point3(1.0, 1.0, 0.25),
        point3(0.0, 1.0, 0.0),
    ]);
    let text = lower_boundary(&model, &UnitScale::default())
        .expect_err("an off-plane boundary point must not lower");
    assert!(text.contains("Degenerate"), "got {text}");
    assert!(
        text.contains("EntityId(12)"),
        "must blame the point: {text}"
    );
    assert!(text.contains("BoundaryDim"), "must name the rule: {text}");
}

/// A composite boundary without segments bounds nothing: refused as
/// degenerate, naming the composite, never routed to a 3D curve the kernel
/// would reject later. Lowered composites are covered by
/// `tests/halfspace_boundary.rs` (#393).
#[test]
fn an_empty_composite_boundary_is_refused_as_degenerate() {
    let mut model = bounded_with_boundary([
        point2(0.0, 0.0),
        point2(1.0, 0.0),
        point2(1.0, 1.0),
        point2(0.0, 1.0),
    ]);
    model.insert(
        EntityId(6),
        entity(
            "IFCCOMPOSITECURVE",
            vec![Value::List(vec![]), Value::Bool(false)],
        ),
    );
    let text = lower_boundary(&model, &UnitScale::default())
        .expect_err("an empty composite boundary must not lower");
    assert!(text.contains("Degenerate"), "got {text}");
    assert!(text.contains("EntityId(6)"), "got {text}");
    assert!(text.contains("IFCCOMPOSITECURVE"), "got {text}");
}

/// A curved base surface is reported, never silently flattened.
///
/// Substituting a tangent plane would cut along the wrong shape and produce a
/// plausible-looking result, so the gap must be named.
#[test]
fn a_non_planar_base_surface_is_reported_as_unsupported() {
    let mut model = half_space(true, 0.0);
    model.insert(
        EntityId(4),
        entity("IFCCYLINDRICALSURFACE", vec![r(3), n(1.0)]),
    );

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let error = lower_half_space_node(&mut session, EntityId(5), Transform::identity())
        .expect_err("a cylindrical base surface must not lower");
    let text = format!("{error:?}");
    assert!(
        text.contains("IFCCYLINDRICALSURFACE"),
        "the error must name the offending surface, got {text}"
    );
}

/// [`super::prism_orientation`] for each relation of `Position` to the plane
/// (#515). A unit `+Z` normal throughout; `Position` varies.
#[test]
fn the_prism_orientation_names_every_frame_relation() {
    use super::{prism_orientation, PrismOrientation};
    use axiolid_core::Vec3;

    let up = Vec3::new(0.0, 0.0, 1.0);
    let frame = |axis: [f64; 3], reference: [f64; 3]| {
        Transform::from_axes([3.0, 4.0, 5.0], Some(axis), Some(reference)).expect("valid axes")
    };
    let cases = [
        (
            frame([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            PrismOrientation::Along,
        ),
        // Turned about the normal: still along.
        (
            frame([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            PrismOrientation::Along,
        ),
        // Upside down: Y = Z x X = -Y, so the plane must be read from below.
        (
            frame([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
            PrismOrientation::Against,
        ),
        // The #515 shape, rotated: the prism is tilted off the normal.
        (
            frame([0.0, 0.6, 0.8], [1.0, 0.0, 0.0]),
            PrismOrientation::Oblique,
        ),
        // A tilt of 1e-9 rad is rounding, not authoring.
        (
            frame([0.0, 1e-9, 1.0], [1.0, 0.0, 0.0]),
            PrismOrientation::Along,
        ),
        (
            Transform::identity().scaled(2.0),
            PrismOrientation::Distorted,
        ),
        (
            Transform::identity().scaled_nonuniform([1.0, 3.0, 1.0]),
            PrismOrientation::Distorted,
        ),
    ];
    for (index, (placed, expected)) in cases.into_iter().enumerate() {
        assert_eq!(prism_orientation(&placed, up), expected, "case {index}");
    }
    // A normal pointing down reverses the sign test, nothing else.
    assert_eq!(
        prism_orientation(&Transform::identity(), -up),
        PrismOrientation::Against
    );
}

/// The `#5` bounded half-space of [`bounded_with_boundary`] with the base
/// plane's normal and `Position`'s Z set, lowered under `frame`.
fn lower_bounded(
    plane_axis: [f64; 3],
    position_axis: [f64; 3],
    frame: Transform,
) -> Result<axiolid_primitive::HalfSpace, crate::error::GeometryError> {
    let mut model = bounded_with_boundary([
        point2(0.0, 0.0),
        point2(1.0, 0.0),
        point2(1.0, 2.0),
        point2(0.0, 2.0),
    ]);
    let direction = |v: [f64; 3]| {
        entity(
            "IFCDIRECTION",
            vec![Value::List(v.iter().map(|c| n(*c)).collect())],
        )
    };
    model.insert(EntityId(2), direction(plane_axis));
    model.insert(EntityId(9), direction(position_axis));
    model.insert(
        EntityId(7),
        entity("IFCAXIS2PLACEMENT3D", vec![r(8), r(9), Value::Null]),
    );
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_half_space_node(&mut session, EntityId(5), frame)?;
    let lowered = session.finish(node).expect("finishes");
    let Some(GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace { half_space, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a BoundedHalfSpace operation");
    };
    let Some(GeometryNode::HalfSpace(hs)) = lowered.graph.get(*half_space) else {
        panic!("expected a HalfSpace plane");
    };
    Ok(*hs)
}

/// A normal against `Position`'s Z is the same half space read from the
/// other side: the node's normal is reversed and its side flipped, so the
/// kernel's `normal x X` is `Position`'s own Y and the footprint is not
/// mirrored (#515).
#[test]
fn a_normal_against_position_z_is_reversed_with_its_side() {
    // `.T.` on a downward normal keeps the material above the plane.
    let hs =
        lower_bounded([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], Transform::identity()).expect("lowers");
    assert_eq!(hs.boundary.normal.to_array(), [0.0, 0.0, 1.0]);
    assert!(hs.agreement, "the kept side is still above the plane");
    // Aligned, the flag converts as for any half space.
    let hs =
        lower_bounded([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], Transform::identity()).expect("lowers");
    assert_eq!(hs.boundary.normal.to_array(), [0.0, 0.0, 1.0]);
    assert!(!hs.agreement);
}

/// The neutral node cannot hold an oblique or rescaled prism: each is
/// refused by name, never lowered to a node that compiles to another tool.
#[test]
fn an_oblique_or_scaled_prism_is_refused_by_name() {
    for (plane, position, frame, needle) in [
        (
            [0.6, 0.0, -0.8],
            [0.0, 0.0, 1.0],
            Transform::identity(),
            "oblique",
        ),
        (
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            Transform::identity().scaled(2.0),
            "scaled or sheared",
        ),
    ] {
        let error = lower_bounded(plane, position, frame).expect_err("must refuse");
        let crate::error::GeometryError::Unsupported {
            entity,
            type_name,
            detail,
        } = &error
        else {
            panic!("expected Unsupported, got {error:?}");
        };
        assert_eq!(*entity, EntityId(5));
        assert_eq!(type_name, "IFCPOLYGONALBOUNDEDHALFSPACE");
        assert!(detail.contains(needle), "{needle}: {detail}");
    }
}

/// Whether `point` is on the kept side of every wall, within `slack`.
fn inside_walls(walls: &[axiolid_primitive::HalfSpace], point: [f64; 3], slack: f64) -> bool {
    walls.iter().all(|wall| {
        assert!(wall.agreement, "a wall keeps its normal side");
        let offset = axiolid_core::Vec3::from_array(point) - wall.boundary.origin;
        offset.dot(wall.boundary.normal) >= -slack
    })
}

/// A convex polygon's walls hold its prism along `Position`'s Z, whichever
/// way it winds, through collinear and repeated vertices, under a frame
/// that tilts, scales or reflects it (#515).
#[test]
fn a_convex_boundary_becomes_one_wall_per_edge() {
    use super::prism_walls;
    use axiolid_core::Point2;

    let square = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)].map(|(x, y)| Point2::new(x, y));
    let mut clockwise = square;
    clockwise.reverse();
    let with_collinear_and_repeat = [
        (0.0, 0.0),
        (1.0, 0.0),
        (2.0, 0.0),
        (2.0, 0.0),
        (2.0, 2.0),
        (0.0, 2.0),
        (0.0, 0.0),
    ]
    .map(|(x, y)| Point2::new(x, y));
    let tilted = Transform::from_axes(
        [1.0, 2.0, 3.0],
        Some([0.0, 0.6, 0.8]),
        Some([1.0, 0.0, 0.0]),
    )
    .expect("valid axes");
    let mut reflected = Transform::identity();
    reflected.basis[1] = [0.0, -1.0, 0.0];
    for (name, frame) in [
        ("identity", Transform::identity()),
        ("tilted", tilted),
        (
            "scaled",
            Transform::identity().scaled_nonuniform([2.0, 3.0, 0.5]),
        ),
        ("reflected", reflected),
    ] {
        for (case, points) in [
            ("counter-clockwise", &square[..]),
            ("clockwise", &clockwise[..]),
            ("collinear and repeated", &with_collinear_and_repeat[..]),
        ] {
            let walls = prism_walls(points, &frame).expect("convex");
            let edges = if points.len() == 4 { 4 } else { 5 };
            assert_eq!(walls.len(), edges, "{name} {case}");
            // The prism's axis through the centre, far along Z both ways, is
            // inside; a point just past each edge's midpoint is not.
            for z in [-1e3, 0.0, 1e3] {
                let centre = frame.apply([1.0, 1.0, z]);
                assert!(inside_walls(&walls, centre, 1e-9), "{name} {case}: z {z}");
            }
            for outside in [[1.0, -0.01], [2.01, 1.0], [1.0, 2.01], [-0.01, 1.0]] {
                let point = frame.apply([outside[0], outside[1], 7.0]);
                assert!(
                    !inside_walls(&walls, point, 0.0),
                    "{name} {case}: {outside:?} is outside"
                );
            }
        }
    }
}

/// A polygon whose edge half-planes do not hold it is refused: a reflex
/// corner, a star that winds twice, no area.
#[test]
fn a_boundary_that_is_not_convex_is_refused() {
    use super::{prism_walls, NON_CONVEX};
    use axiolid_core::Point2;

    let points = |p: &[(f64, f64)]| {
        p.iter()
            .map(|&(x, y)| Point2::new(x, y))
            .collect::<Vec<_>>()
    };
    let star: Vec<(f64, f64)> = (0..5)
        .map(|k| {
            let angle =
                std::f64::consts::FRAC_PI_2 + f64::from(2 * k) * std::f64::consts::TAU / 5.0;
            (angle.cos(), angle.sin())
        })
        .collect();
    for (name, polygon) in [
        (
            "L",
            points(&[
                (0.0, 0.0),
                (2.0, 0.0),
                (2.0, 1.0),
                (1.0, 1.0),
                (1.0, 2.0),
                (0.0, 2.0),
            ]),
        ),
        ("star", points(&star)),
        ("flat", points(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])),
        ("two points", points(&[(0.0, 0.0), (1.0, 0.0)])),
    ] {
        assert_eq!(
            prism_walls(&polygon, &Transform::identity()).expect_err(name),
            NON_CONVEX,
            "{name}"
        );
    }
}

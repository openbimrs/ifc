//! Unit tests for half-space lowering.
//!
//! The assertion that earns its keep is the polarity one: IFC's flag and the
//! kernel's flag are opposites, and getting it wrong produces a boolean that
//! still evaluates and still looks like geometry.

use axiolid_model::GeometryNode;
use ifc_model::{Entity, EntityId, Model, Value};

use super::lower_half_space_node;
use crate::lower::session::LoweringSession;
use crate::lower::tolerance::Tolerance;
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
    let mut session = LoweringSession::new(model, &scale, Tolerance::building_scale());
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

/// Lengths are converted to metres.
#[test]
fn the_plane_origin_is_converted_to_metres() {
    let model = half_space(true, 2500.0);
    let scale = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
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
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_half_space_node(&mut session, EntityId(5), Transform::identity())
        .expect_err("a cylindrical base surface must not lower");
    let text = format!("{error:?}");
    assert!(
        text.contains("IFCCYLINDRICALSURFACE"),
        "the error must name the offending surface, got {text}"
    );
}

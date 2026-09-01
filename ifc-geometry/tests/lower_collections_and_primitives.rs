#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Pyramid, bounding box and the loose-collection families.

use axiolid_model::GeometryNode;
use axiolid_primitive::Primitive;
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn model(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {name} must parse: {e:?}"))
}

fn only(model: &Model, kind: &str) -> EntityId {
    let ids = model.ids_of_type(kind);
    assert_eq!(ids.len(), 1, "{kind}: expected exactly one instance");
    ids[0]
}

/// Lower one item and return the whole graph, so nested nodes stay reachable.
fn lower(model: &Model, id: EntityId) -> ifc_geometry::lower::LoweredGeometry {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .unwrap_or_else(|e| panic!("{id:?} must lower: {e}"));
    session.finish(node).expect("session finishes")
}

fn primitive(model: &Model, kind: &str) -> Primitive {
    let lowered = lower(model, only(model, kind));
    let found = lowered.graph.iter().find_map(|(_, node)| match node {
        GeometryNode::Primitive(value) => Some(*value),
        _ => None,
    });
    found.unwrap_or_else(|| panic!("{kind} must lower as a neutral primitive"))
}

/// The pyramid reads XLength, YLength, Height -- not the cone's slot order.
///
/// `IfcRectangularPyramid` follows `IfcBlock` (Position, then the two base
/// extents, then Height), while `IfcRightCircularCone` puts Height first.
/// Copying the cone arm swaps height with x and still yields a valid pyramid,
/// so the three values must be distinct for this to prove anything: the
/// fixture uses 300 x 200 x 450 mm.
#[test]
fn the_pyramid_reads_its_slots_in_block_order_not_cone_order() {
    let model = model("synthetic_primitives_and_bbox.ifc");
    let lowered = lower(&model, only(&model, "IFCRECTANGULARPYRAMID"));
    let found = lowered.graph.iter().find_map(|(_, n)| match n {
        GeometryNode::Primitive(Primitive::Pyramid { x, y, height }) => Some((*x, *y, *height)),
        _ => None,
    });
    let (x, y, height) = found.expect("a pyramid primitive reaches the graph");
    assert!((x - 0.300).abs() < 1e-9, "XLength 300 mm -> 0.3 m, got {x}");
    assert!((y - 0.200).abs() < 1e-9, "YLength 200 mm -> 0.2 m, got {y}");
    assert!(
        (height - 0.450).abs() < 1e-9,
        "Height 450 mm -> 0.45 m, got {height}"
    );
}

/// Sphere, cylinder, and cone must not collapse onto one slot convention.
#[test]
fn remaining_csg_primitives_keep_their_distinct_dimensions() {
    let model = model("synthetic_primitives_and_bbox.ifc");

    let Primitive::Sphere { radius } = primitive(&model, "IFCSPHERE") else {
        panic!("sphere variant");
    };
    assert!((radius - 0.125).abs() < 1e-9);

    let Primitive::Cylinder { height, radius } = primitive(&model, "IFCRIGHTCIRCULARCYLINDER")
    else {
        panic!("cylinder variant");
    };
    assert!((height - 0.500).abs() < 1e-9);
    assert!((radius - 0.080).abs() < 1e-9);

    let Primitive::Cone { height, radius } = primitive(&model, "IFCRIGHTCIRCULARCONE") else {
        panic!("cone variant");
    };
    assert!((height - 0.450).abs() < 1e-9);
    assert!((radius - 0.120).abs() < 1e-9);
}

/// An identity-placed box passes its corner and extents through unchanged.
#[test]
fn an_unrotated_bounding_box_keeps_its_corner_and_extents() {
    let model = model("synthetic_primitives_and_bbox.ifc");
    let lowered = lower(&model, only(&model, "IFCBOUNDINGBOX"));
    let bounds = lowered
        .graph
        .iter()
        .find_map(|(_, n)| match n {
            GeometryNode::BoundingBox(b) => Some(*b),
            _ => None,
        })
        .expect("a bounding box reaches the graph");
    let min = bounds.min.to_array();
    let max = bounds.max.to_array();
    // Corner (10, 20, 30) mm with extents 100 x 200 x 300 mm.
    for (got, want) in min.iter().zip([0.010, 0.020, 0.030]) {
        assert!((got - want).abs() < 1e-9, "min {min:?} vs expected");
    }
    for (got, want) in max.iter().zip([0.110, 0.220, 0.330]) {
        assert!((got - want).abs() < 1e-9, "max {max:?} vs expected");
    }
}

/// Under rotation the world box is recomputed from all eight corners.
///
/// This is the assertion that pays for the loop. `IfcBoundingBox` is aligned
/// to ITS OWN representation's axes; `GeometryNode::BoundingBox` is an `Aabb`,
/// world-aligned by definition. Transforming only the min and max corners of a
/// rotated box gives a box that is too small and can even invert. A 45-degree
/// rotation about z turns a 100 x 200 mm footprint into a world footprint of
/// (100+200)/sqrt(2) = 212.13 mm on BOTH x and y, while z is untouched.
#[test]
fn a_rotated_bounding_box_grows_to_the_world_aligned_extent() {
    use std::f64::consts::FRAC_1_SQRT_2;
    let model = model("synthetic_primitives_and_bbox.ifc");
    let id = only(&model, "IFCBOUNDINGBOX");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    // 45 degrees about z, applied as the caller's frame.
    let rotated = Transform::from_axes(
        [0.0, 0.0, 0.0],
        Some([0.0, 0.0, 1.0]),
        Some([FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0]),
    )
    .expect("45-degree frame is non-degenerate");
    let node = lower_representation_item(&mut session, id, rotated).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let bounds = lowered
        .graph
        .iter()
        .find_map(|(_, n)| match n {
            GeometryNode::BoundingBox(b) => Some(*b),
            _ => None,
        })
        .expect("a bounding box reaches the graph");
    let dx = bounds.max.to_array()[0] - bounds.min.to_array()[0];
    let dy = bounds.max.to_array()[1] - bounds.min.to_array()[1];
    let dz = bounds.max.to_array()[2] - bounds.min.to_array()[2];
    let expected = (0.100 + 0.200) * FRAC_1_SQRT_2;
    assert!(
        (dx - expected).abs() < 1e-9 && (dy - expected).abs() < 1e-9,
        "rotated footprint must grow to {expected}, got {dx} x {dy}"
    );
    assert!((dz - 0.300).abs() < 1e-9, "z is unaffected by a z-rotation");
}

/// A geometric curve set lowers each member through the same exact curve path.
///
/// The collection must preserve both authored members rather than treating the
/// set as one opaque representation item or dropping standalone curve nodes.
#[test]
fn a_curve_set_preserves_all_dispatched_curve_members() {
    let model = model("synthetic_collections.ifc");
    let id = only(&model, "IFCGEOMETRICCURVESET");
    let lowered = lower(&model, id);
    let members = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Collection(v) => v.len(),
        other => panic!("expected a Collection, got {other:?}"),
    };
    assert_eq!(members, 2, "a polyline and a circle");

    // Both members must be real curve nodes, not placeholders.
    let curves = lowered
        .graph
        .iter()
        .filter(|(_, n)| matches!(n, GeometryNode::Curve3(_)))
        .count();
    assert!(curves >= 2, "both members lower to curves, found {curves}");
}

/// Shell- and face-based surface models stay surfaces: faces, but no solid.
///
/// Both IFC families declare SURFACE models. Even when their shells
/// are closed they are not solids or legal boolean operands. Emitting a
/// solid here would let a quantity takeoff report a volume the file never
/// claimed, so the BRep must carry the shell and leave `solids` empty.
#[test]
fn shell_and_face_based_surface_models_never_acquire_volume() {
    let model = model("synthetic_collections.ifc");
    for kind in ["IFCSHELLBASEDSURFACEMODEL", "IFCFACEBASEDSURFACEMODEL"] {
        let lowered = lower(&model, only(&model, kind));
        match lowered.graph.get(lowered.root).expect("root") {
            GeometryNode::Collection(v) => assert_eq!(v.len(), 1, "{kind}: one face set"),
            other => panic!("{kind}: expected a Collection, got {other:?}"),
        }
        let breps: Vec<_> = lowered
            .graph
            .iter()
            .filter_map(|(_, n)| match n {
                GeometryNode::BRep(b) => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(breps.len(), 1, "{kind}: one neutral BRep");
        assert_eq!(breps[0].faces().len(), 2, "{kind}: two triangles");
        assert!(
            breps[0].solids().is_empty(),
            "{kind}: a surface model must NOT carry a solid"
        );
    }
}

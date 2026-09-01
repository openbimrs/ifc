#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Tessellated face sets lowered from the committed corpus.
//!
//! # Why a fixture and not only synthetic models
//!
//! The unit tests build models by hand, which proves the index arithmetic but
//! not that the reader agrees with a real exporter's slot layout. This walks
//! the committed fixture and asserts the mesh matches what the file declares.

use axiolid_model::GeometryNode;
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn model_of(relative: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(relative);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|error| panic!("fixture {} must parse: {error:?}", path.display()))
}

/// The tetrahedron fixture lowers to 4 shared points and 4 triangles.
///
/// `mapped_instances_indexed_colour.ifc` declares:
/// ```text
/// #8 =IFCCARTESIANPOINTLIST3D(((0,0,0),(1,0,0),(0,1,0),(0,0,1)));
/// #12=IFCTRIANGULATEDFACESET(#8,$,.T.,((1,3,2),(1,2,4),(2,3,4),(3,1,4)),$);
/// ```
/// Four points and four faces: any other count means the shared point list was
/// unshared per corner, which is the failure that silently inflates a mesh.
#[test]
fn the_triangulated_fixture_lowers_to_four_points_and_four_triangles() {
    let model = model_of("ifclite-geometry/mapped_instances_indexed_colour.ifc");
    let scale = units::resolve(&model);
    let id = *model
        .ids_of_type("IFCTRIANGULATEDFACESET")
        .first()
        .expect("the fixture contains a triangulated face set");

    let mut session = LoweringSession::new(&model, &scale);
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .expect("the face set must lower");
    let lowered = session.finish(node).expect("session finishes");

    let mesh = match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::TriMesh(mesh) => mesh,
        other => panic!("expected a TriMesh, got {other:?}"),
    };

    assert_eq!(
        mesh.positions.len(),
        4,
        "four shared points, not 12 corners"
    );
    assert_eq!(mesh.indices.len(), 12, "four triangles");
    assert_eq!(
        &mesh.indices[0..3],
        &[0, 2, 1],
        "the file's first triangle (1,3,2) converted from 1-based"
    );

    // The unit tetrahedron's corners, in file order.
    assert_eq!(mesh.positions[0].to_array(), [0.0, 0.0, 0.0]);
    assert_eq!(mesh.positions[3].to_array(), [0.0, 0.0, 1.0]);
}

/// The polygonal sibling uses referenced face records and the same shared list.
#[test]
fn the_polygonal_fixture_lowers_its_indexed_face_without_unsharing_points() {
    let model = model_of("ifclite-geometry/mapped_instances_indexed_colour.ifc");
    let scale = units::resolve(&model);
    let id = model.ids_of_type("IFCPOLYGONALFACESET")[0];
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_representation_item(&mut session, id, Transform::identity())
        .expect("polygonal set lowers");
    let lowered = session.finish(root).expect("session finishes");
    let GeometryNode::PolygonMesh(mesh) = lowered.graph.get(lowered.root).expect("root") else {
        panic!("polygonal set must preserve authored polygon faces");
    };
    assert_eq!(mesh.positions.len(), 4, "the point list remains shared");
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.faces[0].outer, vec![0, 1, 2]);
    assert!(mesh.faces[0].holes.is_empty());
}

/// Every index a lowered mesh emits addresses a vertex that exists.
///
/// An out-of-range index is not a crash — it is a renderer reading whatever
/// is at that slot. This walks the whole corpus rather than one fixture so a
/// future face-set fixture is covered the day it lands.
#[test]
fn no_lowered_mesh_index_escapes_its_own_position_list() {
    let mut checked = 0usize;
    for relative in [
        "ifclite-geometry/mapped_instances_indexed_colour.ifc",
        "ifclite-geometry/mapped_instances_indexed_colour_uniform.ifc",
    ] {
        let model = model_of(relative);
        let scale = units::resolve(&model);
        for id in model.ids_of_type("IFCTRIANGULATEDFACESET") {
            let mut session = LoweringSession::new(&model, &scale);
            let node = lower_representation_item(&mut session, *id, Transform::identity())
                .expect("face sets in the corpus must lower");
            let lowered = session.finish(node).expect("finishes");
            if let GeometryNode::TriMesh(mesh) = lowered.graph.get(lowered.root).expect("root") {
                let count = mesh.positions.len() as u32;
                for index in &mesh.indices {
                    assert!(
                        *index < count,
                        "{relative}: index {index} addresses past {count} positions"
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 2,
        "expected the face-set fixtures, saw {checked}"
    );
}

/// A placed face set moves; its point list stays shared.
///
/// Applying the frame per corner instead of per position would multiply the
/// point count, which no visual check catches on a small mesh.
#[test]
fn placing_a_face_set_moves_it_without_unsharing_points() {
    let model = model_of("ifclite-geometry/mapped_instances_indexed_colour.ifc");
    let scale = units::resolve(&model);
    let id = *model
        .ids_of_type("IFCTRIANGULATEDFACESET")
        .first()
        .expect("set");

    let mut session = LoweringSession::new(&model, &scale);
    let node =
        lower_representation_item(&mut session, id, Transform::translation([10.0, 20.0, 30.0]))
            .expect("lowers");
    let lowered = session.finish(node).expect("finishes");

    let mesh = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::TriMesh(mesh) => mesh,
        other => panic!("expected a TriMesh, got {other:?}"),
    };
    assert_eq!(mesh.positions.len(), 4, "placement must not unshare points");
    assert_eq!(
        mesh.positions[0].to_array(),
        [10.0, 20.0, 30.0],
        "the origin corner is placed by the frame"
    );
}

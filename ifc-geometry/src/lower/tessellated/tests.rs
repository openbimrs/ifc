//! Unit tests for tessellated face-set lowering.
//!
//! These target the failure modes that survive a visual check: an off-by-one
//! that still renders, a `PnIndex` hop skipped so vertices permute, and
//! normals dragged through a translation. Each produces a mesh that looks
//! plausible and is wrong.

use axiolid_model::GeometryNode;

use super::{lower_polygonal_face_set_node, lower_triangulated_face_set_node};
use crate::lower::dispatch::lower_representation_item;
use crate::lower::session::LoweringSession;
use crate::lower::tolerance::Tolerance;
use crate::solid::testkit::{entity, int_grid as grid, ints, list, model, n, r, refs};
use crate::transform::Transform;
use crate::units::UnitScale;

/// A unit tetrahedron: 4 points, 4 triangles, indices as the file writes them.
fn tetrahedron(pn: Option<&[i64]>) -> ifc_model::Model {
    let coords = entity(
        "IFCCARTESIANPOINTLIST3D",
        vec![list(vec![
            list(vec![n(0.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(0.0), n(0.0)]),
            list(vec![n(0.0), n(1.0), n(0.0)]),
            list(vec![n(0.0), n(0.0), n(1.0)]),
        ])],
    );
    let face_set = entity(
        "IFCTRIANGULATEDFACESET",
        vec![
            r(1),
            ifc_model::Value::Null,
            ifc_model::Value::Bool(true),
            grid(&[&[1, 3, 2], &[1, 2, 4], &[2, 3, 4], &[3, 1, 4]]),
            match pn {
                Some(values) => ints(values),
                None => ifc_model::Value::Null,
            },
        ],
    );
    model(vec![(1, coords), (2, face_set)])
}

fn lower_tri(model: &ifc_model::Model, frame: Transform) -> axiolid_mesh::TriMesh {
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(model, &scale, Tolerance::building_scale());
    let node = lower_triangulated_face_set_node(&mut session, ifc_model::EntityId(2), frame)
        .expect("the face set must lower");
    let lowered = session.finish(node).expect("session finishes");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::TriMesh(mesh) => mesh.clone(),
        other => panic!("expected a TriMesh, got {other:?}"),
    }
}

/// Indices are 1-based in the file and 0-based in the mesh.
///
/// The file's first triangle is `(1,3,2)`. If the conversion is skipped the
/// mesh still builds — it just addresses the wrong vertices, and with four
/// points it does not even go out of bounds. That is why this asserts the
/// exact index triple rather than only the counts.
#[test]
fn coord_index_is_converted_from_one_based_to_zero_based() {
    let mesh = lower_tri(&tetrahedron(None), Transform::identity());

    assert_eq!(mesh.positions.len(), 4, "four shared points");
    assert_eq!(mesh.indices.len(), 12, "four triangles, three corners each");
    assert_eq!(
        &mesh.indices[0..3],
        &[0, 2, 1],
        "file triangle (1,3,2) must become (0,2,1)"
    );
}

/// A `PnIndex` permutation is applied, not ignored.
///
/// `PnIndex` reverses the point order here, so every triangle must address
/// different vertices than the no-PnIndex case. Skipping the hop is invisible
/// in the vertex count and in the triangle count.
#[test]
fn pn_index_indirection_permutes_the_addressed_vertices() {
    let direct = lower_tri(&tetrahedron(None), Transform::identity());
    let permuted = lower_tri(&tetrahedron(Some(&[4, 3, 2, 1])), Transform::identity());

    assert_eq!(
        permuted.positions, direct.positions,
        "PnIndex reorders addressing, never the point list itself"
    );
    assert_eq!(
        &permuted.indices[0..3],
        &[3, 1, 2],
        "through PnIndex [4,3,2,1], file triangle (1,3,2) addresses (3,1,2)"
    );
    assert_ne!(
        direct.indices, permuted.indices,
        "ignoring PnIndex would leave the indices unchanged"
    );
}

/// Positions are translated by the frame; the mesh stays indexed.
#[test]
fn the_frame_places_positions_without_duplicating_them() {
    let frame = Transform::translation([10.0, 0.0, 0.0]);
    let mesh = lower_tri(&tetrahedron(None), frame);

    assert_eq!(mesh.positions.len(), 4, "placement must not unshare points");
    assert_eq!(
        mesh.positions[0].to_array(),
        [10.0, 0.0, 0.0],
        "the origin vertex moves with the frame"
    );
}

/// Normals take the linear part only.
///
/// Sending a normal through the full affine transform adds the translation,
/// which turns a unit normal into a position vector. The mesh still renders;
/// the lighting is wrong everywhere the product is not at the origin.
#[test]
fn normals_are_rotated_but_never_translated() {
    let coords = entity(
        "IFCCARTESIANPOINTLIST3D",
        vec![list(vec![
            list(vec![n(0.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(0.0), n(0.0)]),
            list(vec![n(0.0), n(1.0), n(0.0)]),
        ])],
    );
    let face_set = entity(
        "IFCTRIANGULATEDFACESET",
        vec![
            r(1),
            list(vec![list(vec![n(0.0), n(0.0), n(1.0)])]),
            ifc_model::Value::Bool(true),
            grid(&[&[1, 2, 3]]),
            ifc_model::Value::Null,
        ],
    );
    let model = model(vec![(1, coords), (2, face_set)]);

    let frame = Transform::translation([5.0, 7.0, 9.0]);
    let mesh = lower_tri(&model, frame);
    let normals = mesh.normals.expect("the source declares normals");

    assert_eq!(
        normals.values[0].to_array(),
        [0.0, 0.0, 1.0],
        "a pure translation must leave the normal untouched"
    );
}

/// An index past the end of `Coordinates` is refused, not clamped.
///
/// Substituting the origin would draw a spike to (0,0,0) that looks like
/// geometry. The error must name the offending entity.
#[test]
fn an_index_past_the_end_of_coordinates_is_rejected() {
    let coords = entity(
        "IFCCARTESIANPOINTLIST3D",
        vec![list(vec![
            list(vec![n(0.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(0.0), n(0.0)]),
            list(vec![n(0.0), n(1.0), n(0.0)]),
        ])],
    );
    let face_set = entity(
        "IFCTRIANGULATEDFACESET",
        vec![
            r(1),
            ifc_model::Value::Null,
            ifc_model::Value::Bool(true),
            grid(&[&[1, 2, 9]]),
            ifc_model::Value::Null,
        ],
    );
    let model = model(vec![(1, coords), (2, face_set)]);
    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let error = lower_triangulated_face_set_node(
        &mut session,
        ifc_model::EntityId(2),
        Transform::identity(),
    )
    .expect_err("a dangling index must not lower");
    let text = format!("{error:?}");
    assert!(
        text.contains('9') || text.contains("Coordinates"),
        "the error must name the bad index or the slot, got {text}"
    );
}

/// An n-gon face keeps its authored corner count: no triangulation here.
#[test]
fn polygonal_faces_keep_their_authored_n_gons() {
    let coords = entity(
        "IFCCARTESIANPOINTLIST3D",
        vec![list(vec![
            list(vec![n(0.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(1.0), n(0.0)]),
            list(vec![n(0.0), n(1.0), n(0.0)]),
        ])],
    );
    let face = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3, 4])]);
    let face_set = entity(
        "IFCPOLYGONALFACESET",
        vec![
            r(1),
            ifc_model::Value::Bool(true),
            refs(&[3]),
            ifc_model::Value::Null,
        ],
    );
    let model = model(vec![(1, coords), (3, face), (2, face_set)]);

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_polygonal_face_set_node(&mut session, ifc_model::EntityId(2), Transform::identity())
            .expect("the quad must lower");
    let lowered = session.finish(node).expect("finishes");
    let mesh = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::PolygonMesh(mesh) => mesh.clone(),
        other => panic!("expected a PolygonMesh, got {other:?}"),
    };

    assert_eq!(mesh.faces.len(), 1, "one authored face");
    assert_eq!(
        mesh.faces[0].outer,
        vec![0, 1, 2, 3],
        "a quad stays a quad; triangulating here would report six indices"
    );
    assert!(mesh.faces[0].holes.is_empty(), "no voids declared");
}

/// A face with voids keeps its inner loops separate from the outer one.
///
/// Flattening holes into the outer loop produces a face that tessellates into
/// a filled polygon — the window disappears and the wall looks solid.
#[test]
fn a_face_with_voids_keeps_its_inner_loops() {
    let mut points = Vec::new();
    for (x, y) in [
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 4.0),
        (0.0, 4.0),
        (1.0, 1.0),
        (2.0, 1.0),
        (2.0, 2.0),
        (1.0, 2.0),
    ] {
        points.push(list(vec![n(x), n(y), n(0.0)]));
    }
    let coords = entity("IFCCARTESIANPOINTLIST3D", vec![list(points)]);
    let face = entity(
        "IFCINDEXEDPOLYGONALFACEWITHVOIDS",
        vec![ints(&[1, 2, 3, 4]), list(vec![ints(&[5, 6, 7, 8])])],
    );
    let face_set = entity(
        "IFCPOLYGONALFACESET",
        vec![
            r(1),
            ifc_model::Value::Bool(true),
            refs(&[3]),
            ifc_model::Value::Null,
        ],
    );
    let model = model(vec![(1, coords), (3, face), (2, face_set)]);

    let scale = UnitScale::default();
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node =
        lower_polygonal_face_set_node(&mut session, ifc_model::EntityId(2), Transform::identity())
            .expect("the holed face must lower");
    let lowered = session.finish(node).expect("finishes");
    let mesh = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::PolygonMesh(mesh) => mesh.clone(),
        other => panic!("expected a PolygonMesh, got {other:?}"),
    };

    assert_eq!(mesh.faces[0].outer, vec![0, 1, 2, 3], "the outer boundary");
    assert_eq!(mesh.faces[0].holes.len(), 1, "one void");
    assert_eq!(
        mesh.faces[0].holes[0],
        vec![4, 5, 6, 7],
        "the void keeps its own loop"
    );
}

/// The set-level `PnIndex` hop applies to polygonal faces too.
///
/// `IfcPolygonalFaceSet.PnIndex` remaps what every face's `CoordIndex`
/// addresses. Skipping it leaves a mesh with the right face count, the right
/// vertex count, and silently permuted geometry.
#[test]
fn a_set_level_pn_index_remaps_polygonal_face_indices() {
    fn build(pn: Option<&[i64]>) -> axiolid_mesh::PolygonMesh {
        let coords = entity(
            "IFCCARTESIANPOINTLIST3D",
            vec![list(vec![
                list(vec![n(0.0), n(0.0), n(0.0)]),
                list(vec![n(1.0), n(0.0), n(0.0)]),
                list(vec![n(1.0), n(1.0), n(0.0)]),
                list(vec![n(0.0), n(1.0), n(0.0)]),
            ])],
        );
        let face = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3])]);
        let face_set = entity(
            "IFCPOLYGONALFACESET",
            vec![
                r(1),
                ifc_model::Value::Bool(true),
                refs(&[3]),
                match pn {
                    Some(values) => ints(values),
                    None => ifc_model::Value::Null,
                },
            ],
        );
        let model = model(vec![(1, coords), (3, face), (2, face_set)]);
        let scale = UnitScale::default();
        let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
        let node = lower_polygonal_face_set_node(
            &mut session,
            ifc_model::EntityId(2),
            Transform::identity(),
        )
        .expect("lowers");
        let lowered = session.finish(node).expect("finishes");
        match lowered.graph.get(lowered.root).expect("root") {
            GeometryNode::PolygonMesh(mesh) => mesh.clone(),
            other => panic!("expected a PolygonMesh, got {other:?}"),
        }
    }

    let direct = build(None);
    let permuted = build(Some(&[4, 3, 2, 1]));

    assert_eq!(direct.faces[0].outer, vec![0, 1, 2], "without PnIndex");
    assert_eq!(
        permuted.faces[0].outer,
        vec![3, 2, 1],
        "through PnIndex [4,3,2,1], face (1,2,3) addresses (3,2,1)"
    );
    assert_ne!(
        direct.faces[0].outer, permuted.faces[0].outer,
        "ignoring the set-level PnIndex would leave the loop unchanged"
    );
}

/// Both families are reachable through the total dispatcher.
///
/// The unit tests above call the lowering functions directly, so they still
/// pass if the dispatch arm is deleted and the family silently reverts to
/// `Unsupported`. This routes through `lower_representation_item` instead.
#[test]
fn both_face_set_families_route_through_the_dispatcher() {
    let scale = UnitScale::default();

    let tri = tetrahedron(None);
    let mut session = LoweringSession::new(&tri, &scale, Tolerance::building_scale());
    let node =
        lower_representation_item(&mut session, ifc_model::EntityId(2), Transform::identity())
            .expect("IFCTRIANGULATEDFACESET must dispatch, not report Unsupported");
    let lowered = session.finish(node).expect("finishes");
    assert!(
        matches!(
            lowered.graph.get(lowered.root).expect("root"),
            GeometryNode::TriMesh(_)
        ),
        "the dispatcher must produce a TriMesh"
    );

    let coords = entity(
        "IFCCARTESIANPOINTLIST3D",
        vec![list(vec![
            list(vec![n(0.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(0.0), n(0.0)]),
            list(vec![n(1.0), n(1.0), n(0.0)]),
        ])],
    );
    let face = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3])]);
    let face_set = entity(
        "IFCPOLYGONALFACESET",
        vec![
            r(1),
            ifc_model::Value::Bool(true),
            refs(&[3]),
            ifc_model::Value::Null,
        ],
    );
    let poly = model(vec![(1, coords), (3, face), (2, face_set)]);
    let mut session = LoweringSession::new(&poly, &scale, Tolerance::building_scale());
    let node =
        lower_representation_item(&mut session, ifc_model::EntityId(2), Transform::identity())
            .expect("IFCPOLYGONALFACESET must dispatch, not report Unsupported");
    let lowered = session.finish(node).expect("finishes");
    assert!(
        matches!(
            lowered.graph.get(lowered.root).expect("root"),
            GeometryNode::PolygonMesh(_)
        ),
        "the dispatcher must produce a PolygonMesh"
    );
}

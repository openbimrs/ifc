//! Tessellated meshes: authored, then read back through the same crate.
//!
//! The whole risk here is the 1-based index conversion. A writer that
//! forgets it produces a mesh whose triangles all point at the
//! neighbouring vertex: the file parses, the mesh renders, and it is
//! wrong. So these tests assert on both sides of the boundary -- the
//! raw `Value::Integer` in the record, and the 0-based value the
//! reader hands back.

use ifc_geometry::authoring::{
    cartesian_point_list_2d, cartesian_point_list_3d, indexed_polygonal_face,
    indexed_polygonal_face_with_voids, polygonal_face_set, triangulated_face_set,
    TriangulatedExtras,
};
use ifc_geometry::solid::tessellated::{PolygonalFaceSet, TriangulatedFaceSet};
use ifc_model::{Model, Transaction, Value};

/// A unit tetrahedron: four points, four triangles.
const TETRA: [[f64; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
const FACES: [[usize; 3]; 4] = [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];

/// The authored record stores 1-based indices, and the reader returns
/// the 0-based values that went in. Both halves are asserted: agreeing
/// with the reader alone would pass if both were off by one.
#[test]
fn triangle_indices_survive_the_one_based_boundary() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points = cartesian_point_list_3d(&mut tx, &TETRA, None).expect("points");
    let mesh = triangulated_face_set(
        &mut tx,
        points,
        TETRA.len(),
        &FACES,
        TriangulatedExtras {
            closed: Some(true),
            ..TriangulatedExtras::default()
        },
    )
    .expect("mesh");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(mesh).expect("mesh present");
    let view = TriangulatedFaceSet::new(mesh, entity);

    // The reader's 0-based view returns exactly what was authored.
    let read: Vec<[usize; 3]> = view.triangles_0based().expect("triangles");
    assert_eq!(read, FACES.to_vec());

    // And the stored record really is 1-based: [0,1,2] -> (1,2,3).
    let stored = view.triangles_1based().expect("raw");
    assert_eq!(stored[0], [1, 2, 3]);
    assert_eq!(view.closed(), Some(true));
    assert_eq!(view.triangle_count().expect("count"), 4);
}
/// A polygonal face set round-trips through the face views, and the
/// with-voids variant keeps its inner loops separate from the outer.
#[test]
fn polygonal_faces_and_their_voids_round_trip() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    // A square with a square hole: 4 outer points, 4 inner.
    let points = [
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [4.0, 4.0, 0.0],
        [0.0, 4.0, 0.0],
        [1.0, 1.0, 0.0],
        [2.0, 1.0, 0.0],
        [2.0, 2.0, 0.0],
        [1.0, 2.0, 0.0],
    ];
    let list = cartesian_point_list_3d(&mut tx, &points, None).expect("points");
    let plain = indexed_polygonal_face(&mut tx, &[0, 1, 2, 3], points.len()).expect("face");
    let holed =
        indexed_polygonal_face_with_voids(&mut tx, &[0, 1, 2, 3], &[&[4, 5, 6, 7]], points.len())
            .expect("holed face");
    let set = polygonal_face_set(
        &mut tx,
        list,
        points.len(),
        &[plain, holed],
        Some(false),
        None,
    )
    .expect("face set");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(set).expect("set present");
    let view = PolygonalFaceSet::new(set, entity);
    assert_eq!(view.coordinates().expect("coords"), list);
    assert_eq!(view.closed(), Some(false));
    assert_eq!(view.faces().expect("faces"), vec![plain, holed]);

    // The plain face carries no voids; the other does, and they are
    // stored in a distinct slot rather than appended to the outer loop.
    let plain_entity = model.get(plain).expect("plain present");
    assert_eq!(plain_entity.attributes.len(), 1);
    let holed_entity = model.get(holed).expect("holed present");
    assert_eq!(
        holed_entity.type_name.as_ref(),
        "IFCINDEXEDPOLYGONALFACEWITHVOIDS"
    );
    let Some(Value::List(outer)) = holed_entity.attribute(0) else {
        panic!("outer loop");
    };
    assert_eq!(outer.len(), 4, "the outer loop keeps only its own vertices");
    let Some(Value::List(inner)) = holed_entity.attribute(1) else {
        panic!("inner loops");
    };
    assert_eq!(inner.len(), 1, "one void");
    // 0-based [4,5,6,7] was authored: stored 1-based it starts at 5.
    let Some(Value::List(first)) = inner.first() else {
        panic!("first void");
    };
    assert_eq!(first[0], Value::Integer(5));
}
/// Meshes that would parse but denote nothing are refused.
///
/// The bound check is the load-bearing one: an index past the end of
/// the point list is the failure a round-trip test cannot see, because
/// both directions agree about a vertex that does not exist.
#[test]
fn meshes_that_cannot_denote_a_surface_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points = cartesian_point_list_3d(&mut tx, &TETRA, None).expect("points");

    // 4 points: index 4 is one past the end.
    assert!(
        triangulated_face_set(
            &mut tx,
            points,
            TETRA.len(),
            &[[0, 1, 4]],
            TriangulatedExtras::default()
        )
        .is_err(),
        "an index past the end of the point list"
    );
    assert!(
        triangulated_face_set(
            &mut tx,
            points,
            TETRA.len(),
            &[],
            TriangulatedExtras::default()
        )
        .is_err(),
        "a face set with no triangles"
    );
    assert!(
        indexed_polygonal_face(&mut tx, &[0, 1], TETRA.len()).is_err(),
        "a face through two points bounds no area"
    );
    assert!(
        indexed_polygonal_face_with_voids(&mut tx, &[0, 1, 2], &[], TETRA.len()).is_err(),
        "with-voids without voids is the plain face type"
    );
    assert!(
        indexed_polygonal_face_with_voids(&mut tx, &[0, 1, 2], &[&[0, 1]], TETRA.len()).is_err(),
        "a void loop is held to the same three-vertex minimum"
    );
    assert!(
        cartesian_point_list_3d(&mut tx, &[], None).is_err(),
        "an empty point list"
    );
    assert!(
        cartesian_point_list_3d(&mut tx, &TETRA, Some(&["a", "b"])).is_err(),
        "tags are positional: a short list retags the wrong vertices"
    );
    assert!(
        cartesian_point_list_2d(&mut tx, &[[0.0, f64::NAN]], None).is_err(),
        "a non-finite coordinate"
    );

    let face = indexed_polygonal_face(&mut tx, &[0, 1, 2], TETRA.len()).expect("face");
    assert!(
        polygonal_face_set(&mut tx, points, TETRA.len(), &[face, face], None, None).is_err(),
        "the schema requires a UNIQUE face list"
    );
    assert!(
        polygonal_face_set(&mut tx, points, TETRA.len(), &[], None, None).is_err(),
        "a face set with no faces"
    );
    assert!(
        polygonal_face_set(&mut tx, points, TETRA.len(), &[face], None, Some(&[9])).is_err(),
        "a PnIndex entry outside the point list"
    );
}
/// `TagList` is an IFC4X3 addition. IFC4 declares `CoordList` alone,
/// so an untagged point list must be one attribute wide -- a trailing
/// `$` would be a record IFC4 does not declare.
///
/// The round-trip tests cannot see this: they read the slot they
/// wrote, and a wider record still answers `coordinates()` correctly.
/// Only the arity, checked against the schema, catches it.
#[test]
fn an_untagged_point_list_stays_within_ifc4() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let plain = cartesian_point_list_3d(&mut tx, &TETRA, None).expect("points");
    let tagged =
        cartesian_point_list_3d(&mut tx, &TETRA, Some(&["a", "b", "c", "d"])).expect("tagged");
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(plain).expect("plain").attributes.len(),
        1,
        "IFC4 declares CoordList alone"
    );
    assert_eq!(
        model.get(tagged).expect("tagged").attributes.len(),
        2,
        "TagList appears only when asked for"
    );
}

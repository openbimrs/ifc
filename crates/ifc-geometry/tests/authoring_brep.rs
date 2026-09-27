//! Boundary representation: authored, then read back.
//!
//! The DERIVE handling on IfcOrientedEdge is the case worth guarding:
//! its inherited vertex slots must serialize as `*`, not `$`.

use ifc_geometry::authoring::{
    cartesian_point, direction, edge_curve, face, face_based_surface_model, face_outer_bound, line,
    manifold_solid_brep, oriented_edge, poly_loop, shell, shell_based_surface_model, vector,
    vertex_point, BrepKind, ShellKind,
};
use ifc_geometry::resource::topology::{
    EdgeCurve, Face, FaceBound, ManifoldSolidBrep, OrientedEdge, PolyLoop, VertexPoint,
};
use ifc_model::{Model, Transaction, Value};

/// A tetrahedron's worth of topology, authored bottom up.
#[test]
fn a_faceted_brep_round_trips_through_its_views() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let corners = [
        cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a"),
        cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("b"),
        cartesian_point(&mut tx, &[0.0, 1.0, 0.0]).expect("c"),
    ];
    let loop_id = poly_loop(&mut tx, &corners).expect("loop");
    let bound = face_outer_bound(&mut tx, loop_id, true);
    let face_id = face(&mut tx, &[bound]).expect("face");
    let shell_id = shell(&mut tx, ShellKind::Closed, &[face_id]).expect("shell");
    let brep = manifold_solid_brep(&mut tx, BrepKind::Faceted, shell_id, &[]);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(brep).expect("brep");
    assert_eq!(entity.type_name.as_ref(), "IFCFACETEDBREP");
    let view = ManifoldSolidBrep::new(brep, entity);
    assert_eq!(view.outer().expect("outer"), shell_id);

    let face_entity = model.get(face_id).expect("face");
    let face_view = Face::new(face_id, face_entity);
    let bounds = face_view.bounds().expect("bounds");
    assert_eq!(bounds, vec![bound]);

    let bound_entity = model.get(bound).expect("bound");
    assert_eq!(bound_entity.type_name.as_ref(), "IFCFACEOUTERBOUND");
    let bound_view = FaceBound::new(bound, bound_entity);
    assert_eq!(bound_view.bound().expect("loop"), loop_id);
    assert!(bound_view.orientation().expect("orientation"));

    let loop_entity = model.get(loop_id).expect("loop");
    let loop_view = PolyLoop::new(loop_id, loop_entity);
    assert_eq!(loop_view.polygon().expect("polygon"), corners.to_vec());
}

/// IfcOrientedEdge inherits EdgeStart and EdgeEnd but redeclares them
/// as DERIVE. They must be written `*`, not `$`: both decode to
/// "absent" in Rust, so only the serialized form tells them apart, and
/// a reader that sees `$` is entitled to reject the file.
#[test]
fn an_oriented_edge_states_its_inherited_vertices_as_derived() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let start = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("start");
    let end = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("end");
    let v0 = vertex_point(&mut tx, start);
    let v1 = vertex_point(&mut tx, end);
    let dir = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("dir");
    let along = vector(&mut tx, dir, 1.0).expect("vector");
    let geometry = line(&mut tx, start, along);
    let edge = edge_curve(&mut tx, v0, v1, geometry, true);
    let oriented = oriented_edge(&mut tx, edge, false);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(oriented).expect("oriented edge");
    assert_eq!(entity.type_name.as_ref(), "IFCORIENTEDEDGE");
    assert_eq!(
        entity.attributes[0],
        Value::Derived,
        "EdgeStart is derived on IfcOrientedEdge and must serialize as *"
    );
    assert_eq!(entity.attributes[1], Value::Derived, "EdgeEnd likewise");

    let view = OrientedEdge::new(oriented, entity);
    assert_eq!(view.edge_element().expect("element"), edge);
    assert!(!view.orientation());

    // The underlying edge does state its vertices.
    let edge_entity = model.get(edge).expect("edge");
    let edge_view = EdgeCurve::new(edge, edge_entity);
    assert_eq!(edge_view.start().expect("start"), v0);
    assert_eq!(edge_view.end().expect("end"), v1);
    assert_eq!(edge_view.edge_geometry().expect("geometry"), geometry);
    assert!(edge_view.same_sense());

    let vertex_entity = model.get(v0).expect("vertex");
    let vertex_view = VertexPoint::new(v0, vertex_entity);
    assert_eq!(vertex_view.vertex_geometry().expect("geometry"), start);
}

/// Topology that cannot bound anything is refused at authoring time.
///
/// These are the schema's own cardinalities, not house rules: a loop
/// needs three points, a face needs a bound, a shell needs a face.
#[test]
fn degenerate_topology_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("b");

    assert!(
        poly_loop(&mut tx, &[a, b]).is_err(),
        "two points bound no area; the schema demands LIST [3:?]"
    );
    assert!(
        poly_loop(&mut tx, &[a, b, a]).is_err(),
        "the polygon list is UNIQUE, so a repeated vertex is invalid"
    );
    assert!(
        face(&mut tx, &[]).is_err(),
        "a face needs at least one bound"
    );
    assert!(
        shell(&mut tx, ShellKind::Closed, &[]).is_err(),
        "a shell needs at least one face"
    );
    assert!(
        shell(&mut tx, ShellKind::Connected, &[]).is_err(),
        "a connected face set needs at least one face"
    );
}

/// A brep with voids keeps its cavities distinct from its outer shell,
/// and the advanced form carries the same shape with curved faces.
#[test]
fn voids_and_advanced_breps_keep_their_shells_apart() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let shell_of = |tx: &mut Transaction, z: f64| {
        let a = cartesian_point(tx, &[0.0, 0.0, z]).expect("a");
        let b = cartesian_point(tx, &[1.0, 0.0, z]).expect("b");
        let c = cartesian_point(tx, &[0.0, 1.0, z]).expect("c");
        let l = poly_loop(tx, &[a, b, c]).expect("loop");
        let bound = face_outer_bound(tx, l, true);
        let f = face(tx, &[bound]).expect("face");
        shell(tx, ShellKind::Closed, &[f]).expect("shell")
    };

    let outer = shell_of(&mut tx, 0.0);
    let cavity = shell_of(&mut tx, 0.25);
    let with_voids = manifold_solid_brep(&mut tx, BrepKind::Faceted, outer, &[cavity]);
    let advanced = manifold_solid_brep(&mut tx, BrepKind::Advanced, outer, &[]);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(with_voids).expect("brep");
    assert_eq!(entity.type_name.as_ref(), "IFCFACETEDBREPWITHVOIDS");
    assert_eq!(entity.attributes[0], Value::Ref(outer));
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Ref(cavity)]),
        "the void shell belongs in Voids, never folded into Outer"
    );
    let view = ManifoldSolidBrep::new(with_voids, entity);
    assert_eq!(view.outer().expect("outer"), outer);

    let advanced_entity = model.get(advanced).expect("advanced");
    assert_eq!(advanced_entity.type_name.as_ref(), "IFCADVANCEDBREP");
    assert_eq!(
        ManifoldSolidBrep::new(advanced, advanced_entity)
            .outer()
            .expect("outer"),
        outer
    );
}

/// Surface models carry the same faces without claiming to bound a
/// volume, so an open shell is legal where a brep would need a closed one.
#[test]
fn surface_models_accept_open_shells() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("b");
    let c = cartesian_point(&mut tx, &[0.0, 1.0, 0.0]).expect("c");
    let l = poly_loop(&mut tx, &[a, b, c]).expect("loop");
    let bound = face_outer_bound(&mut tx, l, true);
    let f = face(&mut tx, &[bound]).expect("face");

    let open = shell(&mut tx, ShellKind::Open, &[f]).expect("open shell");
    let shell_model = shell_based_surface_model(&mut tx, &[open]).expect("shell model");
    let set = shell(&mut tx, ShellKind::Connected, &[f]).expect("face set");
    let face_model = face_based_surface_model(&mut tx, &[set]).expect("face model");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(open).expect("open").type_name.as_ref(),
        "IFCOPENSHELL"
    );
    let sm = model.get(shell_model).expect("shell model");
    assert_eq!(sm.type_name.as_ref(), "IFCSHELLBASEDSURFACEMODEL");
    assert_eq!(sm.attributes[0], Value::List(vec![Value::Ref(open)]));

    let fm = model.get(face_model).expect("face model");
    assert_eq!(fm.type_name.as_ref(), "IFCFACEBASEDSURFACEMODEL");
    assert_eq!(fm.attributes[0], Value::List(vec![Value::Ref(set)]));
}

/// An edge is directed: start and end are not interchangeable, and a
/// transposition survives every structural check while reversing the
/// surface normal that depends on it.
#[test]
fn an_edge_keeps_its_direction() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let p0 = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("p0");
    let p1 = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("p1");
    let v0 = vertex_point(&mut tx, p0);
    let v1 = vertex_point(&mut tx, p1);
    let axis = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("axis");
    let along = vector(&mut tx, axis, 1.0).expect("vector");
    let geometry = line(&mut tx, p0, along);
    let edge = edge_curve(&mut tx, v0, v1, geometry, true);
    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(edge).expect("edge present");
    let view = ifc_geometry::resource::topology::EdgeCurve::new(edge, entity);
    assert_eq!(view.start().expect("start"), v0);
    assert_eq!(view.end().expect("end"), v1);
    assert_ne!(
        view.start().expect("start"),
        view.end().expect("end"),
        "a transposition would swap these without changing the arity"
    );
}

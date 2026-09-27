//! Sweep the topology writers and both kind enums.
//!
//! The brep module builds a topology graph: vertices into edges, edges
//! into loops, loops into faces, faces into shells, shells into solids.
//! Tests exercised the paths a few worked examples happened to take, so
//! several writers and several enum arms had never been run -- and a
//! wrong type name or slot count in one of them compiles and passes.
//!
//! Each writer here is called on a real graph rather than in isolation,
//! so a slot that takes the wrong reference type fails.

use ifc_geometry::authoring::{
    edge, edge_loop, face, face_bound, face_outer_bound, face_surface, manifold_solid_brep,
    oriented_edge, poly_loop, shell, subedge, vertex_loop, vertex_point, BrepKind, ShellKind,
};
use ifc_model::{Entity, Model, Transaction, Value};

/// A point to hang vertices on.
fn point(tx: &mut Transaction) -> ifc_model::EntityId {
    tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
        ])],
    ))
}

/// Three distinct points: `IfcPolyLoop.Polygon` is UNIQUE, and a loop
/// closes implicitly, so the same point cannot appear twice.
fn triangle(tx: &mut Transaction) -> [ifc_model::EntityId; 3] {
    [point(tx), point(tx), point(tx)]
}

/// Every topology writer stages its own type name.
#[test]
fn every_topology_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let geometry = point(&mut tx);
    let start = vertex_point(&mut tx, geometry);
    let end = vertex_point(&mut tx, geometry);
    let plain = edge(&mut tx, start, end);
    let sub = subedge(&mut tx, start, end, plain);
    let oriented = oriented_edge(&mut tx, plain, true);
    let loop_of_edges = edge_loop(&mut tx, &[oriented]).expect("edge_loop");
    let corners = triangle(&mut tx);
    let loop_of_points = poly_loop(&mut tx, &corners).expect("poly_loop");
    let loop_of_vertex = vertex_loop(&mut tx, start);
    let inner = face_bound(&mut tx, loop_of_points, true);
    let outer = face_outer_bound(&mut tx, loop_of_points, true);
    let plain_face = face(&mut tx, &[outer, inner]).expect("face");

    tx.commit(&mut model).expect("commit");

    for (id, expected) in [
        (start, "IFCVERTEXPOINT"),
        (plain, "IFCEDGE"),
        (sub, "IFCSUBEDGE"),
        (oriented, "IFCORIENTEDEDGE"),
        (loop_of_edges, "IFCEDGELOOP"),
        (loop_of_points, "IFCPOLYLOOP"),
        (loop_of_vertex, "IFCVERTEXLOOP"),
        (inner, "IFCFACEBOUND"),
        (outer, "IFCFACEOUTERBOUND"),
        (plain_face, "IFCFACE"),
    ] {
        assert_eq!(
            model.get(id).expect("staged").type_name.as_ref(),
            expected,
            "wrong type staged",
        );
    }
}

/// `face_surface` picks its type from the `advanced` flag.
///
/// `IfcAdvancedFace` is the same shape as `IfcFaceSurface` and differs
/// only in claiming an analytic surface, so the flag is the only thing
/// separating them.
#[test]
fn the_advanced_flag_selects_the_face_type() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let corners = triangle(&mut tx);
    let outline = poly_loop(&mut tx, &corners).expect("poly_loop");
    let bound = face_outer_bound(&mut tx, outline, true);
    let surface = tx.create(Entity::new("IFCPLANE", vec![Value::Null; 1]));

    let plain = face_surface(&mut tx, &[bound], surface, true, false).expect("face_surface");
    let advanced = face_surface(&mut tx, &[bound], surface, true, true).expect("face_surface");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(plain).expect("staged").type_name.as_ref(),
        "IFCFACESURFACE",
    );
    assert_eq!(
        model.get(advanced).expect("staged").type_name.as_ref(),
        "IFCADVANCEDFACE",
    );
}

/// Every `ShellKind` arm stages its own type.
#[test]
fn every_shell_kind_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let corners = triangle(&mut tx);
    let outline = poly_loop(&mut tx, &corners).expect("poly_loop");
    let bound = face_outer_bound(&mut tx, outline, true);
    let a_face = face(&mut tx, &[bound]).expect("face");

    let cases = [
        (ShellKind::Closed, "IFCCLOSEDSHELL"),
        (ShellKind::Open, "IFCOPENSHELL"),
        (ShellKind::Connected, "IFCCONNECTEDFACESET"),
    ];
    let staged: Vec<_> = cases
        .iter()
        .map(|(kind, _)| shell(&mut tx, *kind, &[a_face]).expect("shell"))
        .collect();
    tx.commit(&mut model).expect("commit");

    for (id, (_, expected)) in staged.iter().zip(cases) {
        assert_eq!(model.get(*id).expect("staged").type_name.as_ref(), expected);
    }
}

/// `manifold_solid_brep` crosses the kind with the presence of voids.
///
/// Four type names come out of two booleans, and the `WithVoids` forms
/// were never staged.
#[test]
fn the_brep_kinds_cross_with_voids() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let corners = triangle(&mut tx);
    let outline = poly_loop(&mut tx, &corners).expect("poly_loop");
    let bound = face_outer_bound(&mut tx, outline, true);
    let a_face = face(&mut tx, &[bound]).expect("face");
    let outer = shell(&mut tx, ShellKind::Closed, &[a_face]).expect("shell");
    let void = shell(&mut tx, ShellKind::Closed, &[a_face]).expect("shell");

    let cases = [
        (BrepKind::Faceted, false, "IFCFACETEDBREP"),
        (BrepKind::Faceted, true, "IFCFACETEDBREPWITHVOIDS"),
        (BrepKind::Advanced, false, "IFCADVANCEDBREP"),
        (BrepKind::Advanced, true, "IFCADVANCEDBREPWITHVOIDS"),
    ];
    let staged: Vec<_> = cases
        .iter()
        .map(|(kind, with_voids, _)| {
            let voids: &[_] = if *with_voids { &[void] } else { &[] };
            manifold_solid_brep(&mut tx, *kind, outer, voids)
        })
        .collect();
    tx.commit(&mut model).expect("commit");

    for (id, (_, _, expected)) in staged.iter().zip(cases) {
        assert_eq!(model.get(*id).expect("staged").type_name.as_ref(), expected);
    }
}

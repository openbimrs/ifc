//! Placement, connection geometry, grids and geometric sets.

use ifc_geometry::authoring::{
    axis2_placement_3d, boolean_clipping_result, cartesian_point, circle, connection_geometry,
    connection_point_eccentricity, direction, edge_curve, geometric_set, grid_axis, grid_placement,
    line, local_placement, path, plane, point_on_curve, point_on_surface, polyline,
    rectangle_profile, sectioned_spine, vector, vertex_point, virtual_grid_intersection,
    ConnectionKind,
};
use ifc_model::{EntityId, Model, Transaction, Value};

/// A placement at the origin.
fn origin(tx: &mut Transaction) -> EntityId {
    let point = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("point");
    axis2_placement_3d(tx, point, None, None)
}

/// Local placements chain; an absent parent means absolute.
#[test]
fn local_placements_chain_through_their_parent() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);

    let root = local_placement(&mut tx, None, at);
    let child = local_placement(&mut tx, Some(root), at);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(root).expect("root");
    assert_eq!(
        entity.attributes[0],
        Value::Null,
        "an absent parent is absolute placement, not a missing reference"
    );
    assert_eq!(entity.attributes[1], Value::Ref(at));

    let entity = model.get(child).expect("child");
    assert_eq!(entity.attributes[0], Value::Ref(root), "PlacementRelTo");
}

/// A grid intersection needs exactly two distinct axes.
#[test]
fn a_grid_intersection_needs_two_distinct_axes() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[10.0, 0.0, 0.0]).expect("b");
    let along = polyline(&mut tx, &[a, b]).expect("curve");

    let u = grid_axis(&mut tx, Some("A"), along, true);
    let v = grid_axis(&mut tx, Some("1"), along, false);

    assert!(
        virtual_grid_intersection(&mut tx, &[u], &[0.0, 0.0]).is_err(),
        "LIST [2:2] needs two axes"
    );
    assert!(
        virtual_grid_intersection(&mut tx, &[u, v, u], &[0.0, 0.0]).is_err(),
        "and no more than two"
    );
    assert!(
        virtual_grid_intersection(&mut tx, &[u, u], &[0.0, 0.0]).is_err(),
        "the list is UNIQUE; an axis does not cross itself"
    );
    assert!(
        virtual_grid_intersection(&mut tx, &[u, v], &[0.0]).is_err(),
        "LIST [2:3] offsets"
    );
    assert!(
        virtual_grid_intersection(&mut tx, &[u, v], &[0.0, 0.0, 0.0, 0.0]).is_err(),
        "at most three offsets"
    );

    let crossing =
        virtual_grid_intersection(&mut tx, &[u, v], &[0.25, -0.5]).expect("intersection");
    let placed = grid_placement(&mut tx, None, crossing, None);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(crossing).expect("crossing");
    assert_eq!(
        entity.attributes[0],
        Value::List(vec![Value::Ref(u), Value::Ref(v)])
    );
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Real(0.25), Value::Real(-0.5)]),
        "offsets are signed lengths"
    );

    let entity = model.get(placed).expect("placed");
    assert_eq!(entity.attributes[1], Value::Ref(crossing));

    // The axis tag and sense survive.
    let entity = model.get(v).expect("axis");
    assert_eq!(entity.attributes[0], Value::Text("1".into()));
    assert_eq!(entity.attributes[2], Value::Bool(false), "SameSense");
}

/// Connection geometry keeps the two sides distinct.
///
/// An absent `on_related` means both elements share the geometry; it
/// is not the same as repeating the first reference, and a consumer
/// can tell the difference.
#[test]
fn connection_geometry_distinguishes_its_two_sides() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("b");
    let v0 = vertex_point(&mut tx, a);
    let v1 = vertex_point(&mut tx, b);
    let flat = plane(&mut tx, at);
    let arc = circle(&mut tx, at, 1.0).expect("arc");

    let shared = connection_geometry(&mut tx, ConnectionKind::Point, v0, None);
    let split = connection_geometry(&mut tx, ConnectionKind::Point, v0, Some(v1));
    let curve = connection_geometry(&mut tx, ConnectionKind::Curve, arc, None);
    let surface = connection_geometry(&mut tx, ConnectionKind::Surface, flat, None);

    let eccentric =
        connection_point_eccentricity(&mut tx, v0, Some(v1), [Some(0.05), None, Some(-0.1)])
            .expect("eccentricity");
    assert!(
        connection_point_eccentricity(&mut tx, v0, None, [Some(f64::NAN), None, None]).is_err(),
        "a non-finite eccentricity"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(shared).expect("shared");
    assert_eq!(entity.type_name.as_ref(), "IFCCONNECTIONPOINTGEOMETRY");
    assert_eq!(
        entity.attributes[1],
        Value::Null,
        "both sides share the geometry; that is not the same as repeating it"
    );

    let entity = model.get(split).expect("split");
    assert_eq!(entity.attributes[0], Value::Ref(v0));
    assert_eq!(entity.attributes[1], Value::Ref(v1));
    assert_ne!(entity.attributes[0], entity.attributes[1]);

    assert_eq!(
        model.get(curve).expect("curve").type_name.as_ref(),
        "IFCCONNECTIONCURVEGEOMETRY"
    );
    assert_eq!(
        model.get(surface).expect("surface").type_name.as_ref(),
        "IFCCONNECTIONSURFACEGEOMETRY"
    );

    // Eccentricities are signed and individually optional.
    let entity = model.get(eccentric).expect("eccentric");
    assert_eq!(entity.attributes[2], Value::Real(0.05), "X");
    assert_eq!(entity.attributes[3], Value::Null, "Y was not stated");
    assert_eq!(entity.attributes[4], Value::Real(-0.1), "Z, and signed");
}

/// Points on curves and surfaces keep their parameter measures.
#[test]
fn parametric_points_keep_their_measures() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);
    let arc = circle(&mut tx, at, 1.0).expect("arc");

    let on_curve = point_on_curve(&mut tx, arc, 0.25).expect("on curve");
    // Distinct u and v, so a transposition is visible.
    let on_surface = point_on_surface(&mut tx, flat, 0.25, 0.75).expect("on surface");
    assert!(
        point_on_curve(&mut tx, arc, f64::INFINITY).is_err(),
        "a non-finite parameter"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let unwrap =
        |id: EntityId, index: usize| match &model.get(id).expect("entity").attributes[index] {
            Value::Typed { type_name, value } => {
                assert_eq!(type_name.as_ref(), "IFCPARAMETERVALUE");
                match **value {
                    Value::Real(v) => v,
                    ref other => panic!("not a real: {other:?}"),
                }
            }
            other => panic!("slot {index} lost its measure: {other:?}"),
        };
    assert_eq!(unwrap(on_curve, 1), 0.25);
    assert_eq!(unwrap(on_surface, 1), 0.25, "PointParameterU");
    assert_eq!(unwrap(on_surface, 2), 0.75, "PointParameterV");
}

/// A clipping result is always a difference against a half space.
///
/// `OperatorType` fixes the operator, so the writer takes no operator
/// argument. A union of a solid with a half space is the universe,
/// not a clip.
#[test]
fn a_clipping_result_is_always_a_difference() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);
    let solid = ifc_geometry::authoring::block(&mut tx, at, 1.0, 1.0, 1.0).expect("block");
    let cut = ifc_geometry::authoring::half_space(&mut tx, flat, true);

    let clipped = boolean_clipping_result(&mut tx, solid, cut);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(clipped).expect("clipped");
    assert_eq!(
        entity.attributes[0],
        Value::Enum("DIFFERENCE".into()),
        "OperatorType admits nothing else"
    );
    assert_eq!(entity.attributes[1], Value::Ref(solid), "FirstOperand");
    assert_eq!(entity.attributes[2], Value::Ref(cut), "SecondOperand");
}

/// A spine needs a position for every cross section.
#[test]
fn a_sectioned_spine_pairs_sections_with_positions() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[10.0, 0.0, 0.0]).expect("b");
    let spine = polyline(&mut tx, &[a, b]).expect("spine");
    let first = rectangle_profile(&mut tx, Some("A"), None, 0.3, 0.2).expect("first");
    let second = rectangle_profile(&mut tx, Some("B"), None, 0.2, 0.1).expect("second");

    assert!(
        sectioned_spine(&mut tx, spine, &[first], &[at]).is_err(),
        "LIST [2:?]: one section is not a spine"
    );
    assert!(
        sectioned_spine(&mut tx, spine, &[first, second], &[at]).is_err(),
        "a section with no position places nothing"
    );

    let id = sectioned_spine(&mut tx, spine, &[first, second], &[at, at]).expect("spine");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("spine");
    assert_eq!(entity.attributes[0], Value::Ref(spine), "SpineCurve");
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Ref(first), Value::Ref(second)])
    );
    assert_eq!(
        entity.attributes[2],
        Value::List(vec![Value::Ref(at), Value::Ref(at)])
    );
}

/// Geometric sets and paths enforce their list rules.
#[test]
fn geometric_sets_and_paths_enforce_their_lists() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[1.0, 0.0, 0.0]).expect("b");
    let dir = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("dir");
    let along = vector(&mut tx, dir, 1.0).expect("vector");
    let geometry = line(&mut tx, a, along);
    let v0 = vertex_point(&mut tx, a);
    let v1 = vertex_point(&mut tx, b);
    let edge = edge_curve(&mut tx, v0, v1, geometry, true);
    let oriented = ifc_geometry::authoring::oriented_edge(&mut tx, edge, true);

    assert!(geometric_set(&mut tx, false, &[]).is_err(), "SET [1:?]");
    let plain = geometric_set(&mut tx, false, &[arc]).expect("set");
    let curves = geometric_set(&mut tx, true, &[arc]).expect("curve set");

    assert!(path(&mut tx, &[]).is_err(), "LIST [1:?]");
    assert!(
        path(&mut tx, &[oriented, oriented]).is_err(),
        "the edge list is UNIQUE"
    );
    let run = path(&mut tx, &[oriented]).expect("path");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(plain).expect("plain").type_name.as_ref(),
        "IFCGEOMETRICSET"
    );
    assert_eq!(
        model.get(curves).expect("curves").type_name.as_ref(),
        "IFCGEOMETRICCURVESET"
    );
    assert_eq!(
        model.get(run).expect("path").attributes[0],
        Value::List(vec![Value::Ref(oriented)])
    );
}

//! Bare topology, bare profiles, and the sectioned sweeps.

use ifc_geometry::authoring::{
    bare_topology, cartesian_point, cartesian_point_list_3d, direction, line, open_cross_profile,
    profile_def, rectangle_profile, sectioned, triangulated_irregular_network, BareTopology,
    ProfileType, SectionedKind, TriangulatedExtras,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn directrix(tx: &mut Transaction) -> EntityId {
    let origin = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("origin");
    let dir = direction(tx, &[1.0, 0.0, 0.0]).expect("direction");
    line(tx, origin, dir)
}

fn linear_placement(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCAXIS2PLACEMENTLINEAR", vec![Value::Null; 3]))
}

/// The attribute-less topological supertypes stage.
///
/// Both are concrete in the schema. They exist so a partial model that
/// names a connection without its geometry can round-trip.
#[test]
fn the_bare_topology_items_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let loop_id = bare_topology(&mut tx, BareTopology::Loop);
    let vertex = bare_topology(&mut tx, BareTopology::Vertex);
    tx.commit(&mut model).expect("commit");

    let staged = model.get(loop_id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLOOP");
    assert!(staged.attributes.is_empty(), "IfcLoop has no attributes");

    let staged = model.get(vertex).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCVERTEX");
    assert!(staged.attributes.is_empty(), "IfcVertex has no attributes");
}

/// The bare profile supertype stages with its two slots.
#[test]
fn the_bare_profile_def_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = profile_def(&mut tx, ProfileType::Area, Some("Section"));
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCPROFILEDEF");
    assert_eq!(staged.attributes.len(), 2);
    assert_eq!(staged.attributes[0], Value::Enum("AREA".into()));
}

/// An open cross-section pairs each width with a slope.
///
/// CorrespondingSlopeWidths ties the two lists, and CorrespondingTags
/// makes the tag list one longer: tags name the points between
/// segments, so n segments have n+1 of them.
#[test]
fn an_open_cross_profile_pairs_widths_with_slopes() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    assert!(
        open_cross_profile(&mut tx, None, true, &[1.0, 2.0], &[0.1], None, None).is_err(),
        "mismatched widths and slopes were accepted",
    );
    assert!(
        open_cross_profile(&mut tx, None, true, &[], &[], None, None).is_err(),
        "an empty LIST [1:?] was accepted",
    );
    assert!(
        open_cross_profile(&mut tx, None, true, &[-1.0], &[0.1], None, None).is_err(),
        "a negative non-negative-length measure was accepted",
    );
    assert!(
        open_cross_profile(
            &mut tx,
            None,
            true,
            &[1.0, 2.0],
            &[0.1, 0.2],
            Some(&["a", "b"]),
            None,
        )
        .is_err(),
        "two tags were accepted for two segments; three are required",
    );

    let id = open_cross_profile(
        &mut tx,
        Some("Carriageway"),
        true,
        &[3.5, 3.5],
        &[0.025, -0.025],
        Some(&["left", "crown", "right"]),
        None,
    )
    .expect("open cross profile");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCOPENCROSSPROFILEDEF");
    assert_eq!(staged.attributes.len(), 7);
    // CorrectProfileType fixes this to CURVE: an open chain bounds no area.
    assert_eq!(staged.attributes[0], Value::Enum("CURVE".into()));
}

/// The sectioned solid and surface swap slots 1 and 2.
///
/// Same three attributes, different order: the solid is Directrix,
/// CrossSections, CrossSectionPositions; the surface is Directrix,
/// CrossSectionPositions, CrossSections. Writing one layout under the
/// other type name parses cleanly and is wrong, which is why the
/// writer selects the order rather than the caller.
#[test]
fn the_sectioned_pair_order_their_slots_differently() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = directrix(&mut tx);
    let a = rectangle_profile(&mut tx, None, None, 1.0, 1.0).expect("profile a");
    let b = rectangle_profile(&mut tx, None, None, 2.0, 2.0).expect("profile b");
    let p1 = linear_placement(&mut tx);
    let p2 = linear_placement(&mut tx);

    let solid = sectioned(
        &mut tx,
        SectionedKind::SolidHorizontal,
        curve,
        &[a, b],
        &[p1, p2],
    )
    .expect("sectioned solid");
    let surface = sectioned(&mut tx, SectionedKind::Surface, curve, &[a, b], &[p1, p2])
        .expect("sectioned surface");
    tx.commit(&mut model).expect("commit");

    let sections = Value::List(vec![Value::Ref(a), Value::Ref(b)]);
    let positions = Value::List(vec![Value::Ref(p1), Value::Ref(p2)]);

    let staged = model.get(solid).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSECTIONEDSOLIDHORIZONTAL");
    assert_eq!(staged.attributes[1], sections, "solid: sections first");
    assert_eq!(staged.attributes[2], positions);

    let staged = model.get(surface).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSECTIONEDSURFACE");
    assert_eq!(staged.attributes[1], positions, "surface: positions first");
    assert_eq!(staged.attributes[2], sections);
}

/// Both require at least two sections, one position each.
#[test]
fn a_sectioned_sweep_needs_matched_pairs() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = directrix(&mut tx);
    let a = rectangle_profile(&mut tx, None, None, 1.0, 1.0).expect("profile");
    let p1 = linear_placement(&mut tx);
    let p2 = linear_placement(&mut tx);

    for kind in [SectionedKind::SolidHorizontal, SectionedKind::Surface] {
        assert!(
            sectioned(&mut tx, kind, curve, &[a], &[p1]).is_err(),
            "a single cross-section was accepted; LIST [2:?] requires two",
        );
        assert!(
            sectioned(&mut tx, kind, curve, &[a, a], &[p1, p2, p1]).is_err(),
            "mismatched section and position counts were accepted",
        );
    }
}

/// A TIN carries one flag per triangle and is never closed.
///
/// NotClosed forces Closed to FALSE: a terrain surface is a height
/// field, so it cannot enclose a volume and the caller does not choose.
#[test]
fn a_terrain_network_stages_unclosed() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let coords = cartesian_point_list_3d(
        &mut tx,
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.5],
            [1.0, 1.0, 0.5],
        ],
        None,
    )
    .expect("coordinates");
    let triangles = [[0, 1, 2], [1, 3, 2]];

    assert!(
        triangulated_irregular_network(
            &mut tx,
            coords,
            4,
            &triangles,
            TriangulatedExtras::default(),
            &[0],
        )
        .is_err(),
        "one flag was accepted for two triangles",
    );

    let id = triangulated_irregular_network(
        &mut tx,
        coords,
        4,
        &triangles,
        TriangulatedExtras {
            closed: Some(true),
            ..TriangulatedExtras::default()
        },
        &[0, 1],
    )
    .expect("terrain network");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCTRIANGULATEDIRREGULARNETWORK");
    assert_eq!(staged.attributes.len(), 6);
    assert_eq!(
        staged.attributes[2],
        Value::Bool(false),
        "NotClosed overrides the caller",
    );
    assert_eq!(
        staged.attributes[5],
        Value::List(vec![Value::Integer(0), Value::Integer(1)]),
        "Flags is slot 5",
    );
}

/// The directrix-derived sweep shares the fixed-reference layout.
///
/// Same six slots; the reference is derived from the directrix rather
/// than held constant, so the profile rotates as it sweeps.
#[test]
fn the_directrix_derived_sweep_stages() {
    use ifc_geometry::authoring::{
        directrix_derived_reference_swept_area_solid, fixed_reference_swept_area_solid, SweepTrim,
    };

    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = directrix(&mut tx);
    let profile = rectangle_profile(&mut tx, None, None, 1.0, 1.0).expect("profile");
    let reference = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("reference");

    let fixed = fixed_reference_swept_area_solid(
        &mut tx,
        profile,
        None,
        curve,
        SweepTrim::default(),
        reference,
    )
    .expect("fixed reference sweep");
    let derived = directrix_derived_reference_swept_area_solid(
        &mut tx,
        profile,
        None,
        curve,
        SweepTrim::default(),
        reference,
    )
    .expect("directrix derived sweep");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(derived).expect("staged");
    assert_eq!(
        staged.type_name.as_ref(),
        "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
    );
    assert_eq!(staged.attributes.len(), 6);
    assert_eq!(staged.attributes[5], Value::Ref(reference));
    // Identical layout to its sibling, which is why they share a body.
    let sibling = model.get(fixed).expect("staged");
    assert_eq!(staged.attributes.len(), sibling.attributes.len());
    assert_eq!(staged.attributes[0], sibling.attributes[0]);
}

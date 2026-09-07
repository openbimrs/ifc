//! Space boundaries, including the subtype forms real exports write.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::relation::boundary;
use ifc_spatial::{BoundaryExposure, BoundaryPhysicality};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// A boundary of the given concrete type.
///
/// Slots: 0-3 IfcRoot, 4 RelatingSpace, 5 RelatedBuildingElement,
/// 6 ConnectionGeometry, 7 PhysicalOrVirtual, 8 InternalOrExternal,
/// 9 ParentBoundary, 10 CorrespondingBoundary.
fn boundary_attrs(
    space: EntityId,
    element: EntityId,
    physical: &str,
    exposure: &str,
    parent: Option<EntityId>,
    corresponding: Option<EntityId>,
) -> Vec<Value> {
    let mut a = vec![Value::Null; 4];
    a.push(Value::Ref(space));
    a.push(Value::Ref(element));
    a.push(Value::Null);
    a.push(Value::Enum(physical.into()));
    a.push(Value::Enum(exposure.into()));
    a.push(parent.map_or(Value::Null, Value::Ref));
    a.push(corresponding.map_or(Value::Null, Value::Ref));
    a
}

/// The trap this module exists for: a second-level boundary must be found by
/// a plain query, not silently skipped because its type name is not the
/// supertype's.
#[test]
fn subtype_boundaries_are_found_not_skipped() {
    let mut model = Model::new();
    let space = put(&mut model, 1, "IFCSPACE", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);

    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY",
        boundary_attrs(space, wall, "PHYSICAL", "INTERNAL", None, None),
    );
    put(
        &mut model,
        11,
        "IFCRELSPACEBOUNDARY1STLEVEL",
        boundary_attrs(space, wall, "PHYSICAL", "INTERNAL", None, None),
    );
    put(
        &mut model,
        12,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(space, wall, "PHYSICAL", "EXTERNAL", None, None),
    );

    let found = boundary::all(&model);
    assert_eq!(
        found.len(),
        3,
        "all three concrete types must be read, got {found:?}"
    );
    let mut names: Vec<_> = found.iter().map(|b| b.type_name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "IFCRELSPACEBOUNDARY",
            "IFCRELSPACEBOUNDARY1STLEVEL",
            "IFCRELSPACEBOUNDARY2NDLEVEL"
        ]
    );
}

#[test]
fn ends_and_enumerations_are_read_as_stated() {
    let mut model = Model::new();
    let space = put(&mut model, 1, "IFCSPACE", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY",
        boundary_attrs(space, wall, "VIRTUAL", "EXTERNAL_EARTH", None, None),
    );

    let b = &boundary::all(&model)[0];
    assert_eq!(b.space, Some(space));
    assert_eq!(b.element, Some(wall));
    assert_eq!(b.physicality, BoundaryPhysicality::Virtual);
    // EXTERNAL_EARTH must not collapse into External: ground contact is a
    // different heat-transfer path.
    assert_eq!(b.exposure, BoundaryExposure::ExternalVariant);
    assert_ne!(b.exposure, BoundaryExposure::External);
}

/// Second-level pairing is the point of second-level boundaries.
#[test]
fn parent_and_corresponding_links_resolve() {
    let mut model = Model::new();
    let space_a = put(&mut model, 1, "IFCSPACE", vec![]);
    let space_b = put(&mut model, 2, "IFCSPACE", vec![]);
    let wall = put(&mut model, 3, "IFCWALL", vec![]);
    let window = put(&mut model, 4, "IFCWINDOW", vec![]);

    let outer = EntityId(10);
    let other_side = EntityId(12);
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(
            space_a,
            wall,
            "PHYSICAL",
            "INTERNAL",
            None,
            Some(other_side),
        ),
    );
    // An inner boundary: the window within the wall's boundary.
    put(
        &mut model,
        11,
        "IFCRELSPACEBOUNDARY1STLEVEL",
        boundary_attrs(space_a, window, "PHYSICAL", "INTERNAL", Some(outer), None),
    );
    put(
        &mut model,
        12,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(space_b, wall, "PHYSICAL", "INTERNAL", None, Some(outer)),
    );

    let all = boundary::all(&model);
    let inner = all.iter().find(|b| b.id == EntityId(11)).unwrap();
    assert_eq!(inner.parent, Some(outer), "inner boundary names its parent");

    let first = all.iter().find(|b| b.id == EntityId(10)).unwrap();
    assert_eq!(first.corresponding, Some(other_side));
    let second = all.iter().find(|b| b.id == EntityId(12)).unwrap();
    assert_eq!(second.corresponding, Some(outer), "pairing is symmetric");
}

/// A plain IfcRelSpaceBoundary has no ParentBoundary slot at all. Reading
/// past the end of its attribute list must yield None, not panic.
#[test]
fn plain_boundary_has_no_parent_slot() {
    let mut model = Model::new();
    let space = put(&mut model, 1, "IFCSPACE", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    let mut short = vec![Value::Null; 4];
    short.push(Value::Ref(space));
    short.push(Value::Ref(wall));
    // Stops at slot 5: no geometry, no enums, no parent.
    put(&mut model, 10, "IFCRELSPACEBOUNDARY", short);

    let b = &boundary::all(&model)[0];
    assert_eq!(b.parent, None);
    assert_eq!(b.corresponding, None);
    // Absent enumeration reads as NotDefined, matching the schema's own
    // member rather than inventing a default.
    assert_eq!(b.physicality, BoundaryPhysicality::NotDefined);
    assert_eq!(b.exposure, BoundaryExposure::NotDefined);
}

/// CorrectPhysOrVirt, reported and not enforced.
#[test]
fn physicality_disagreement_is_reported_not_rejected() {
    let mut model = Model::new();
    let space = put(&mut model, 1, "IFCSPACE", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    let virtual_element = put(&mut model, 3, "IFCVIRTUALELEMENT", vec![]);
    let opening = put(&mut model, 4, "IFCOPENINGELEMENT", vec![]);

    // Physical naming a real wall: agrees.
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY",
        boundary_attrs(space, wall, "PHYSICAL", "INTERNAL", None, None),
    );
    // Physical naming a virtual element: violates CorrectPhysOrVirt.
    put(
        &mut model,
        11,
        "IFCRELSPACEBOUNDARY",
        boundary_attrs(space, virtual_element, "PHYSICAL", "INTERNAL", None, None),
    );
    // Virtual naming an opening: the schema allows this.
    put(
        &mut model,
        12,
        "IFCRELSPACEBOUNDARY",
        boundary_attrs(space, opening, "VIRTUAL", "INTERNAL", None, None),
    );

    let all = boundary::all(&model);
    let check = |id: u64| {
        all.iter()
            .find(|b| b.id == EntityId(id))
            .unwrap()
            .physical_matches_element(&model)
    };
    assert_eq!(check(10), Some(true));
    assert_eq!(check(11), Some(false), "a violation is reported...");
    assert_eq!(check(12), Some(true), "...an opening is legal for Virtual");

    // ...and the violating boundary is still returned. This crate reports
    // what the file says; rejecting belongs to ifc-validate.
    assert_eq!(all.len(), 3);
}

#[test]
fn queries_by_space_and_by_element() {
    let mut model = Model::new();
    let space_a = put(&mut model, 1, "IFCSPACE", vec![]);
    let space_b = put(&mut model, 2, "IFCSPACE", vec![]);
    let wall = put(&mut model, 3, "IFCWALL", vec![]);
    let slab = put(&mut model, 4, "IFCSLAB", vec![]);

    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(space_a, wall, "PHYSICAL", "INTERNAL", None, None),
    );
    put(
        &mut model,
        11,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(space_b, wall, "PHYSICAL", "INTERNAL", None, None),
    );
    put(
        &mut model,
        12,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        boundary_attrs(space_a, slab, "PHYSICAL", "EXTERNAL", None, None),
    );

    assert_eq!(boundary::of_space(&model, space_a).len(), 2);
    assert_eq!(boundary::of_space(&model, space_b).len(), 1);
    // A wall between two rooms is named by both sides' boundaries.
    assert_eq!(boundary::of_element(&model, wall).len(), 2);
    assert_eq!(boundary::of_element(&model, slab).len(), 1);
}

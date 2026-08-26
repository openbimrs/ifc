//! Building the containment tree, including from files that break the rules.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{SpatialKind, SpatialTree};

/// Insert an entity at a chosen id so relationships can name it.
fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// Four IfcRoot slots, then the two ends.
fn rel(relating: Option<EntityId>, related: &[EntityId], relating_first: bool) -> Vec<Value> {
    let relating_value = relating.map_or(Value::Null, Value::Ref);
    let related_value = Value::List(related.iter().copied().map(Value::Ref).collect());
    let mut attributes = vec![Value::Null; 4];
    if relating_first {
        attributes.push(relating_value);
        attributes.push(related_value);
    } else {
        attributes.push(related_value);
        attributes.push(relating_value);
    }
    attributes
}

/// project -> site -> building -> storey, with two walls on the storey.
fn canonical() -> (
    Model,
    EntityId,
    EntityId,
    EntityId,
    EntityId,
    EntityId,
    EntityId,
) {
    let mut model = Model::new();
    let project = put(&mut model, 1, "IFCPROJECT", vec![]);
    let site = put(&mut model, 2, "IFCSITE", vec![]);
    let building = put(&mut model, 3, "IFCBUILDING", vec![]);
    let storey = put(&mut model, 4, "IFCBUILDINGSTOREY", vec![]);
    let wall_a = put(&mut model, 5, "IFCWALL", vec![]);
    let wall_b = put(&mut model, 6, "IFCWALL", vec![]);

    put(
        &mut model,
        10,
        "IFCRELAGGREGATES",
        rel(Some(project), &[site], true),
    );
    put(
        &mut model,
        11,
        "IFCRELAGGREGATES",
        rel(Some(site), &[building], true),
    );
    put(
        &mut model,
        12,
        "IFCRELAGGREGATES",
        rel(Some(building), &[storey], true),
    );
    // Containment uses the INVERTED slot order.
    put(
        &mut model,
        13,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        rel(Some(storey), &[wall_a, wall_b], false),
    );
    (model, project, site, building, storey, wall_a, wall_b)
}

#[test]
fn the_canonical_hierarchy_assembles() {
    let (model, project, site, building, storey, wall_a, wall_b) = canonical();
    let tree = SpatialTree::build(&model);

    assert_eq!(tree.roots(), [project], "the project is the only root");
    assert_eq!(tree.node(site).unwrap().parent, Some(project));
    assert_eq!(tree.node(building).unwrap().parent, Some(site));
    assert_eq!(tree.node(storey).unwrap().parent, Some(building));
    assert_eq!(tree.elements_of(storey), [wall_a, wall_b]);
    assert!(tree.orphans().is_empty());
    assert!(tree.dangling().is_empty());
}

/// The inversion trap: if containment slots were read like aggregation slots,
/// the wall would become the parent of the storey.
#[test]
fn containment_is_not_inverted() {
    let (model, _, _, _, storey, wall_a, _) = canonical();
    let tree = SpatialTree::build(&model);

    assert_eq!(
        tree.container_of(wall_a),
        Some(storey),
        "wall is IN the storey"
    );
    assert!(
        tree.node(wall_a).is_none(),
        "a wall is not a container and gets no node"
    );
    assert!(
        !tree.elements_of(wall_a).contains(&storey),
        "the storey must never be an element of the wall"
    );
}

#[test]
fn ancestors_walk_up_to_the_project() {
    let (model, project, site, building, storey, _, _) = canonical();
    let tree = SpatialTree::build(&model);
    assert_eq!(tree.ancestors(storey), [building, site, project]);
    assert!(
        tree.ancestors(project).is_empty(),
        "the root has no ancestors"
    );
}

#[test]
fn elements_recursive_descends_through_spaces() {
    let (mut model, _, _, _, storey, wall_a, wall_b) = canonical();
    let space = put(&mut model, 7, "IFCSPACE", vec![]);
    let door = put(&mut model, 8, "IFCDOOR", vec![]);
    put(
        &mut model,
        14,
        "IFCRELAGGREGATES",
        rel(Some(storey), &[space], true),
    );
    put(
        &mut model,
        15,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        rel(Some(space), &[door], false),
    );

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.elements_of(storey), [wall_a, wall_b], "direct only");
    assert_eq!(
        tree.elements_recursive(storey),
        [wall_a, wall_b, door],
        "own elements precede nested ones"
    );
}

/// Real exports omit the site and hang the building off the project.
#[test]
fn an_omitted_level_still_produces_one_tree() {
    let mut model = Model::new();
    let project = put(&mut model, 1, "IFCPROJECT", vec![]);
    let building = put(&mut model, 2, "IFCBUILDING", vec![]);
    let storey = put(&mut model, 3, "IFCBUILDINGSTOREY", vec![]);
    put(
        &mut model,
        10,
        "IFCRELAGGREGATES",
        rel(Some(project), &[building], true),
    );
    put(
        &mut model,
        11,
        "IFCRELAGGREGATES",
        rel(Some(building), &[storey], true),
    );

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.roots(), [project]);
    assert_eq!(
        tree.ancestors(storey),
        [building, project],
        "site is simply absent"
    );
    assert!(tree.orphans().is_empty());
}

/// Elements attached directly to the building, not to a storey.
#[test]
fn elements_may_hang_off_any_container() {
    let mut model = Model::new();
    let building = put(&mut model, 1, "IFCBUILDING", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    put(
        &mut model,
        10,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        rel(Some(building), &[wall], false),
    );

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.elements_of(building), [wall]);
    assert_eq!(tree.container_of(wall), Some(building));
}

/// A container nothing aggregates is a detached branch, reported not dropped.
#[test]
fn a_detached_container_is_reported_as_an_orphan() {
    let (mut model, project, _, _, _, _, _) = canonical();
    let stray = put(&mut model, 20, "IFCBUILDING", vec![]);

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.roots().first(), Some(&project), "project still leads");
    assert!(tree.roots().contains(&stray));
    assert_eq!(tree.orphans(), [stray], "the stray building is flagged");
    assert!(
        tree.node(stray).is_some(),
        "and is still present rather than silently dropped"
    );
}

/// A relationship naming an entity the file does not contain.
#[test]
fn a_dangling_relationship_is_reported_not_fatal() {
    let (mut model, _, _, _, storey, wall_a, wall_b) = canonical();
    let ghost = EntityId(999);
    put(
        &mut model,
        21,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        rel(Some(storey), &[ghost], false),
    );

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.dangling(), [(EntityId(21), ghost)]);
    assert_eq!(
        tree.elements_of(storey),
        [wall_a, wall_b],
        "the real elements survive"
    );
}

/// Two containment relationships naming the same wall: first container wins,
/// deterministically, rather than the element appearing on two storeys.
#[test]
fn a_duplicated_element_gets_one_stable_container() {
    let (mut model, _, _, building, storey, wall_a, _) = canonical();
    let second = put(&mut model, 22, "IFCBUILDINGSTOREY", vec![]);
    put(
        &mut model,
        23,
        "IFCRELAGGREGATES",
        rel(Some(building), &[second], true),
    );
    put(
        &mut model,
        24,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        rel(Some(second), &[wall_a], false),
    );

    let tree = SpatialTree::build(&model);
    assert_eq!(tree.container_of(wall_a), Some(storey), "first wins");
    let appearances = tree
        .containers()
        .filter(|node| node.elements.contains(&wall_a))
        .count();
    assert_eq!(
        appearances, 2,
        "both relationships are still visible in the tree"
    );
}

/// An aggregation cycle must not hang the walk.
#[test]
fn a_containment_cycle_terminates() {
    let mut model = Model::new();
    let a = put(&mut model, 1, "IFCBUILDING", vec![]);
    let b = put(&mut model, 2, "IFCBUILDINGSTOREY", vec![]);
    put(&mut model, 10, "IFCRELAGGREGATES", rel(Some(a), &[b], true));
    put(&mut model, 11, "IFCRELAGGREGATES", rel(Some(b), &[a], true));

    let tree = SpatialTree::build(&model);
    // Whichever edge is applied first wins; the point is that this returns.
    let ancestors = tree.ancestors(b);
    assert!(ancestors.len() <= 2, "walk stopped: {ancestors:?}");
    assert!(tree.elements_recursive(a).is_empty());
}

/// An element aggregating its parts is valid IFC but not spatial containment.
#[test]
fn element_decomposition_does_not_enter_the_spatial_tree() {
    let mut model = Model::new();
    let stair = put(&mut model, 1, "IFCSTAIR", vec![]);
    let flight = put(&mut model, 2, "IFCSTAIRFLIGHT", vec![]);
    put(
        &mut model,
        10,
        "IFCRELAGGREGATES",
        rel(Some(stair), &[flight], true),
    );

    let tree = SpatialTree::build(&model);
    assert!(
        tree.node(stair).is_none(),
        "a stair is not a spatial container"
    );
    assert!(tree.roots().is_empty());
}

#[test]
fn ifc4x3_spatial_types_are_recognised_without_a_version_list() {
    assert!(SpatialKind::classify("IFCSPATIALZONE").is_container());
    assert!(SpatialKind::classify("IFCFACILITY").is_container());
    assert_eq!(SpatialKind::classify("IFCWALL"), SpatialKind::Element);
    assert_eq!(
        SpatialKind::classify("ifcbuildingstorey"),
        SpatialKind::Storey
    );
}

//! `IfcRelReferencedInSpatialStructure`: an element referenced by several
//! structures, kept apart from the one structure containing it.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::SpatialTree;

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// Four `IfcRoot` slots, then `RelatedElements` (4) and the structure (5):
/// the layout both spatial relationships share.
fn placed(structure: EntityId, elements: &[EntityId]) -> Vec<Value> {
    let mut attributes = vec![Value::Null; 4];
    attributes.push(Value::List(
        elements.iter().copied().map(Value::Ref).collect(),
    ));
    attributes.push(Value::Ref(structure));
    attributes
}

fn aggregates(parent: EntityId, children: &[EntityId]) -> Vec<Value> {
    let mut attributes = vec![Value::Null; 4];
    attributes.push(Value::Ref(parent));
    attributes.push(Value::List(
        children.iter().copied().map(Value::Ref).collect(),
    ));
    attributes
}

/// building -> storeys 3 and 4; a curtain wall contained by 3 and
/// referenced by 4; a column contained by 4.
fn two_storeys() -> Model {
    let mut model = Model::new();
    let building = put(&mut model, 1, "IFCBUILDING", vec![]);
    let ground = put(&mut model, 3, "IFCBUILDINGSTOREY", vec![]);
    let upper = put(&mut model, 4, "IFCBUILDINGSTOREY", vec![]);
    let curtain = put(&mut model, 5, "IFCCURTAINWALL", vec![]);
    let column = put(&mut model, 6, "IFCCOLUMN", vec![]);
    put(
        &mut model,
        10,
        "IFCRELAGGREGATES",
        aggregates(building, &[ground, upper]),
    );
    put(
        &mut model,
        11,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        placed(ground, &[curtain]),
    );
    put(
        &mut model,
        12,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        placed(upper, &[column]),
    );
    put(
        &mut model,
        13,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(upper, &[curtain]),
    );
    model
}

#[test]
fn a_referenced_element_is_listed_apart_from_containment() {
    let tree = SpatialTree::build(&two_storeys());
    let (ground, upper, curtain, column) = (EntityId(3), EntityId(4), EntityId(5), EntityId(6));

    assert_eq!(tree.elements_of(ground), [curtain]);
    assert_eq!(tree.elements_of(upper), [column], "containment only");
    assert_eq!(tree.referenced_elements(upper), [curtain]);
    assert!(tree.referenced_elements(ground).is_empty());

    assert_eq!(tree.container_of(curtain), Some(ground));
    assert_eq!(tree.referencing_structures(curtain), [upper]);
    assert!(tree.referencing_structures(column).is_empty());
    assert!(
        tree.anomalies().is_empty(),
        "a reference is not a second home"
    );
}

#[test]
fn references_keep_file_order_and_repeat_nothing() {
    let mut model = two_storeys();
    let ground = EntityId(3);
    let (curtain, column) = (EntityId(5), EntityId(6));
    // A second relationship repeats the curtain wall and adds the column.
    put(
        &mut model,
        14,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(ground, &[column, curtain]),
    );
    put(
        &mut model,
        15,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(EntityId(4), &[curtain]),
    );
    let tree = SpatialTree::build(&model);
    assert_eq!(tree.referenced_elements(ground), [column, curtain]);
    assert_eq!(tree.referenced_elements(EntityId(4)), [curtain]);
    assert_eq!(tree.referencing_structures(curtain), [EntityId(4), ground]);
}

#[test]
fn a_reference_to_an_absent_entity_is_dangling() {
    let mut model = two_storeys();
    put(
        &mut model,
        16,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(EntityId(4), &[EntityId(99)]),
    );
    put(
        &mut model,
        17,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(EntityId(98), &[EntityId(6)]),
    );
    let tree = SpatialTree::build(&model);
    assert_eq!(
        tree.dangling(),
        [(EntityId(16), EntityId(99)), (EntityId(17), EntityId(98))]
    );
    assert_eq!(tree.referenced_elements(EntityId(4)), [EntityId(5)]);
}

#[test]
fn a_reference_whose_structure_is_not_a_container_is_ignored() {
    let mut model = two_storeys();
    // The column is not a spatial structure: no tree node to list under.
    put(
        &mut model,
        18,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        placed(EntityId(6), &[EntityId(5)]),
    );
    let tree = SpatialTree::build(&model);
    assert!(tree.referenced_elements(EntityId(6)).is_empty());
    assert_eq!(tree.referencing_structures(EntityId(5)), [EntityId(4)]);
}

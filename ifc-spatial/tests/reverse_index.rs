use ifc_model::{Entity, EntityId, Model, Value};
use ifc_spatial::{relation, RelationshipIndex, RelationshipKind};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let id = EntityId(id);
    model.insert(id, Entity::new(type_name, attributes));
    id
}

fn relationship_attributes(relating: EntityId, related: &[EntityId]) -> Vec<Value> {
    vec![
        Value::Null,
        Value::Null,
        Value::Null,
        Value::Null,
        Value::Ref(relating),
        Value::List(related.iter().copied().map(Value::Ref).collect()),
    ]
}

#[test]
fn reusable_index_matches_scan_for_repeated_inverse_queries() {
    let mut model = Model::new();
    let storey = put(&mut model, 1, "IFCBUILDINGSTOREY", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    let door = put(&mut model, 3, "IFCDOOR", vec![]);
    let aggregate = put(
        &mut model,
        10,
        "IFCRELAGGREGATES",
        relationship_attributes(storey, &[wall, door]),
    );
    let nests = put(
        &mut model,
        11,
        "IFCRELNESTS",
        relationship_attributes(storey, &[wall]),
    );

    // Same slot and target, but not a relationship owned by this domain.
    put(
        &mut model,
        12,
        "IFCUNRELATED",
        relationship_attributes(storey, &[wall]),
    );
    // The target appears only at the relating end and must not be returned by
    // the inverse "which relationship names me as a child" query.
    put(
        &mut model,
        13,
        "IFCRELAGGREGATES",
        relationship_attributes(wall, &[storey]),
    );

    let index = RelationshipIndex::build(&model);
    assert_eq!(index.naming(wall), relation::naming(&model, wall));
    assert_eq!(index.naming(door), relation::naming(&model, door));
    assert_eq!(
        index
            .naming(wall)
            .into_iter()
            .map(|relationship| (relationship.id, relationship.kind))
            .collect::<Vec<_>>(),
        [
            (aggregate, RelationshipKind::Aggregates),
            (nests, RelationshipKind::Nests),
        ]
    );
}

#[test]
fn containment_inverse_uses_its_asymmetric_related_slot() {
    let mut model = Model::new();
    let storey = put(&mut model, 1, "IFCBUILDINGSTOREY", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    let contained = put(
        &mut model,
        10,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(wall)]),
            Value::Ref(storey),
        ],
    );

    let index = RelationshipIndex::build(&model);
    assert_eq!(index.naming(wall)[0].id, contained);
    assert!(index.naming(storey).is_empty());
}

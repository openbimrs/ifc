//! The reverse-reference index.

use ifc_model::{Entity, EntityId, Model, Referrer, ReverseIndex, Value};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

#[test]
fn referrers_record_the_slot_the_reference_sat_in() {
    let mut model = Model::new();
    let target = put(&mut model, 1, "IFCWALL", vec![]);
    let holder = put(
        &mut model,
        2,
        "IFCRELVOIDSELEMENT",
        vec![Value::Null, Value::Ref(target)],
    );

    let index = ReverseIndex::build(&model);
    assert_eq!(
        index.referrers(target),
        [Referrer {
            from: holder,
            slot: 1
        }]
    );
    assert!(index.is_referenced(target));
    assert!(!index.is_referenced(holder));
}

/// Slots matter because objectified relationships put the two ends in
/// different attributes; "who references me" alone cannot tell them apart.
#[test]
fn referrers_in_slot_separates_the_two_ends_of_a_relationship() {
    let mut model = Model::new();
    let storey = put(&mut model, 1, "IFCBUILDINGSTOREY", vec![]);
    let wall = put(&mut model, 2, "IFCWALL", vec![]);
    // slot 4 = related elements, slot 5 = relating structure
    let rel = put(
        &mut model,
        3,
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

    let index = ReverseIndex::build(&model);
    assert_eq!(
        index.referrers_in_slot(storey, 5).collect::<Vec<_>>(),
        [rel]
    );
    assert!(index.referrers_in_slot(storey, 4).next().is_none());
    assert_eq!(index.referrers_in_slot(wall, 4).collect::<Vec<_>>(), [rel]);
}

#[test]
fn nested_references_report_the_outermost_slot() {
    let mut model = Model::new();
    let target = put(&mut model, 1, "IFCCARTESIANPOINT", vec![]);
    let holder = put(
        &mut model,
        2,
        "IFCPOLYLINE",
        vec![Value::List(vec![Value::List(vec![Value::Ref(target)])])],
    );

    let index = ReverseIndex::build(&model);
    assert_eq!(
        index.referrers(target),
        [Referrer {
            from: holder,
            slot: 0
        }],
        "the schema names slot 0, not the nesting inside it"
    );
}

#[test]
fn a_repeated_reference_in_one_slot_is_reported_once() {
    let mut model = Model::new();
    let target = put(&mut model, 1, "IFCCARTESIANPOINT", vec![]);
    put(
        &mut model,
        2,
        "IFCPOLYLINE",
        vec![Value::List(vec![Value::Ref(target), Value::Ref(target)])],
    );

    let index = ReverseIndex::build(&model);
    assert_eq!(index.referrers(target).len(), 1, "deduplicated per slot");
}

#[test]
fn an_unreferenced_target_yields_an_empty_slice() {
    let model = Model::new();
    let index = ReverseIndex::build(&model);
    assert!(index.referrers(EntityId(42)).is_empty());
    assert!(index.is_empty());
}

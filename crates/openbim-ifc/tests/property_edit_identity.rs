//! The `GlobalId`s a property edit derives are reproducible from the seed and
//! the edits, whatever entity ids the model allocates (#559), and a seed
//! names one batch.
//!
//! The model is one `IfcBuildingElementProxy` (`#7`) with an owner history;
//! the sets are custom, so no catalog is needed.

#![cfg(all(feature = "step", feature = "properties"))]

use std::collections::BTreeSet;

use ifc::{
    apply_property_edits, Codec, Entity, EntityId, Model, PropertyEdit, PropertyEditFailure,
    StepCodec, Transaction, Value,
};

fn model() -> Model {
    let text = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCPERSON($,'P',$,$,$,$,$,$);
#2=IFCORGANIZATION($,'O',$,$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1','A','A');
#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);
#7=IFCBUILDINGELEMENTPROXY('2YvctVUKr0kugbFTf53O9L',#5,'P',$,$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;
";
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn label(text: &str) -> Value {
    Value::Typed {
        type_name: "IFCLABEL".into(),
        value: Box::new(Value::Text(text.into())),
    }
}

fn set(model: &mut Model, set: &str, name: &str, seed: u128) -> Vec<EntityId> {
    apply_property_edits(
        model,
        &[PropertyEdit::set(EntityId(7), set, name, label("a"))],
        seed,
    )
    .expect("applies")
    .created
}

fn remove(model: &mut Model, set: &str, name: &str, seed: u128) {
    apply_property_edits(model, &[PropertyEdit::remove(EntityId(7), set, name)], seed)
        .expect("applies");
}

fn roundtrip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("writes");
    StepCodec.read_bytes(&bytes).expect("reads")
}

fn global_id(model: &Model, id: EntityId) -> String {
    match model.get(id).and_then(|entity| entity.attributes.first()) {
        Some(Value::Text(text)) => text.to_string(),
        other => panic!("{id}: no GlobalId, {other:?}"),
    }
}

/// `(type, GlobalId)` of every property set and its relationship.
fn identities(model: &Model) -> BTreeSet<(String, String)> {
    model
        .iter()
        .filter(|(_, entity)| {
            matches!(
                &*entity.type_name,
                "IFCPROPERTYSET" | "IFCRELDEFINESBYPROPERTIES"
            )
        })
        .map(|(id, entity)| (entity.type_name.to_string(), global_id(model, id)))
        .collect()
}

fn journal(model: &mut Model) {
    set(model, "Custom", "A", 1);
    remove(model, "Custom", "A", 2);
}

fn property_set(model: &Model, created: &[EntityId]) -> EntityId {
    *created
        .iter()
        .find(|id| {
            model
                .get(**id)
                .is_some_and(|e| &*e.type_name == "IFCPROPERTYSET")
        })
        .expect("a set was created")
}

#[test]
fn a_replayed_journal_gives_the_same_global_ids_after_a_write_and_read() {
    let mut a = model();
    journal(&mut a);
    let a_next = a.next_id();
    set(&mut a, "Custom", "B", 3);

    let mut b = model();
    journal(&mut b);
    let mut b = roundtrip(&b);
    assert_ne!(
        a_next,
        b.next_id(),
        "precondition: the write/read cycle shifts the next free id"
    );
    set(&mut b, "Custom", "B", 3);

    assert_eq!(identities(&a), identities(&b));
    assert_eq!(identities(&a).len(), 2);
}

#[test]
fn global_ids_do_not_depend_on_entity_ids() {
    let mut a = model();
    let mut b = model();
    b.push(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
        ])],
    ));
    journal(&mut a);
    journal(&mut b);
    set(&mut a, "Custom", "B", 3);
    set(&mut b, "Custom", "B", 3);
    assert_eq!(identities(&a), identities(&b));
    assert_eq!(identities(&a).len(), 2);
}

#[test]
fn a_seed_whose_global_id_is_held_is_refused() {
    let mut m = model();
    let created = set(&mut m, "Custom", "A", 559);
    let id = property_set(&m, &created);
    let held = global_id(&m, id);
    let mut tx = Transaction::new(&m);
    tx.set_attribute(id, 2, Value::Text("Renamed".into()));
    tx.commit(&mut m).expect("renames");
    let (bytes, revision) = (StepCodec.write_bytes(&m).expect("writes"), m.revision());

    let error = apply_property_edits(
        &mut m,
        &[PropertyEdit::set(EntityId(7), "Custom", "A", label("b"))],
        559,
    )
    .expect_err("refused");
    assert_eq!(error.edit, Some(0));
    match error.failure {
        PropertyEditFailure::SeedReused { global_id, holder } => {
            assert_eq!(global_id, held);
            assert_eq!(holder, id);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(StepCodec.write_bytes(&m).expect("writes"), bytes);
    assert_eq!(m.revision(), revision);

    set(&mut m, "Custom", "A", 560);
}

#[test]
fn a_seed_whose_records_were_removed_may_be_used_again() {
    let mut m = model();
    let created = set(&mut m, "Custom", "A", 559);
    let first = global_id(&m, property_set(&m, &created));
    remove(&mut m, "Custom", "A", 600);
    let created = set(&mut m, "Custom", "A", 559);
    assert_eq!(global_id(&m, property_set(&m, &created)), first);
}

#[test]
fn a_seed_may_edit_another_set() {
    let mut m = model();
    set(&mut m, "Custom", "A", 559);
    set(&mut m, "Other", "A", 559);
    assert_eq!(identities(&m).len(), 4);
}

#[test]
fn records_removed_in_the_same_batch_do_not_hold_the_seed() {
    let mut m = model();
    let created = set(&mut m, "Custom", "A", 559);
    let id = property_set(&m, &created);
    let mut tx = Transaction::new(&m);
    tx.set_attribute(id, 2, Value::Text("Renamed".into()));
    tx.commit(&mut m).expect("renames");
    let outcome = apply_property_edits(
        &mut m,
        &[
            PropertyEdit::remove(EntityId(7), "Renamed", "A"),
            PropertyEdit::set(EntityId(7), "Custom", "A", label("b")),
        ],
        559,
    )
    .expect("the removed set does not hold the seed");
    assert!(outcome.removed.contains(&id));
}

#[test]
fn two_objects_sharing_a_global_id_get_distinct_record_ids() {
    let mut m = model();
    let mut proxy = m.get(EntityId(7)).expect("proxy").clone();
    proxy.attributes[2] = Value::Text("Q".into());
    let twin = m.push(proxy);
    let outcome = apply_property_edits(
        &mut m,
        &[
            PropertyEdit::set(EntityId(7), "Custom", "A", label("a")),
            PropertyEdit::set(twin, "Custom", "A", label("a")),
        ],
        559,
    )
    .expect("applies");
    assert_eq!(
        outcome
            .created
            .iter()
            .filter(|id| {
                m.get(**id).is_some_and(|e| {
                    matches!(
                        &*e.type_name,
                        "IFCPROPERTYSET" | "IFCRELDEFINESBYPROPERTIES"
                    )
                })
            })
            .count(),
        4
    );
    assert_eq!(identities(&m).len(), 4);
}

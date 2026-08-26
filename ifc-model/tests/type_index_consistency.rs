//! `insert` replaces an occupant; the type index must follow it.
//!
//! Regression coverage: the index previously appended unconditionally, so
//! re-inserting under an existing id listed that id twice, and replacing an
//! entity with one of a different type left it listed under both. Any edit
//! layer built on `insert` inherits those as silent wrong answers.

use ifc_model::value::EntityId;
use ifc_model::{Entity, Model, Value};

#[test]
fn reinserting_the_same_type_does_not_duplicate_the_index() {
    let mut model = Model::new();
    let id = EntityId(1);
    model.insert(id, Entity::new("IFCWALL", vec![Value::Null]));
    model.insert(
        id,
        Entity::new("IFCWALL", vec![Value::Text("edited".into())]),
    );

    assert_eq!(model.len(), 1, "one entity is stored");
    assert_eq!(model.ids_of_type("IFCWALL"), [id], "listed exactly once");
    assert_eq!(model.ids().count(), 1, "file order holds one id");
    assert_eq!(
        model.get(id).unwrap().text(0),
        Some("edited"),
        "the replacement won"
    );
}

#[test]
fn replacing_with_another_type_clears_the_old_index_entry() {
    let mut model = Model::new();
    let id = EntityId(1);
    model.insert(id, Entity::new("IFCWALL", vec![]));
    model.insert(id, Entity::new("IFCDOOR", vec![]));

    assert!(
        model.ids_of_type("IFCWALL").is_empty(),
        "no longer a wall, so must not be indexed as one"
    );
    assert_eq!(model.ids_of_type("IFCDOOR"), [id]);
}

#[test]
fn the_histogram_agrees_with_the_index_after_replacement() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(2), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(1), Entity::new("IFCDOOR", vec![]));

    assert_eq!(
        model.type_histogram(),
        [("IFCDOOR", 1), ("IFCWALL", 1)],
        "counts must not double-count a replaced entity; ties sort by name"
    );
}

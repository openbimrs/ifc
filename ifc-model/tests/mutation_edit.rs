//! Behavioral coverage for the checked edit operations on `Model`.
//!
//! These are the operations `docs/adr/0007` reserves for `ifc-model` itself
//! (schema-agnostic edits), as distinct from `ifc-author`'s schema-checked
//! construction.

use ifc_model::{Entity, EntityId, Model, Value};

fn point(x: f64, y: f64) -> Entity {
    Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(x), Value::Real(y)])],
    )
}

#[test]
fn set_attribute_replaces_the_slot_and_returns_the_previous_value() {
    let mut model = Model::new();
    let id = model.push(point(0.0, 0.0));

    let previous = model.set_attribute(
        id,
        0,
        Value::List(vec![Value::Real(10.0), Value::Real(20.0)]),
    );

    assert_eq!(
        previous,
        Some(Value::List(vec![Value::Real(0.0), Value::Real(0.0)]))
    );
    let coords = model
        .get(id)
        .unwrap()
        .attribute(0)
        .unwrap()
        .as_list()
        .unwrap();
    assert_eq!(coords[0].as_f64(), Some(10.0));
    assert_eq!(coords[1].as_f64(), Some(20.0));
}

#[test]
fn set_attribute_on_a_missing_id_returns_none_and_touches_nothing() {
    let mut model = Model::new();
    let result = model.set_attribute(EntityId(999), 0, Value::Real(1.0));
    assert_eq!(result, None);
    assert_eq!(model.len(), 0);
}

#[test]
fn set_attribute_past_the_current_arity_pads_with_null() {
    let mut model = Model::new();
    let id = model.push(Entity::new(
        "IFCWALL",
        vec![Value::Text("only slot".into())],
    ));

    model.set_attribute(id, 3, Value::Text("far slot".into()));

    let entity = model.get(id).unwrap();
    assert_eq!(entity.attributes.len(), 4);
    assert_eq!(entity.attribute(1), Some(&Value::Null));
    assert_eq!(entity.attribute(2), Some(&Value::Null));
    assert_eq!(entity.text(3), Some("far slot"));
}

#[test]
fn set_attributes_applies_every_edit_and_reports_the_originals_in_order() {
    let mut model = Model::new();
    let id = model.push(Entity::new(
        "IFCWALL",
        vec![
            Value::Text("guid".into()),
            Value::Null,
            Value::Text("Interior wall".into()),
        ],
    ));

    let previous = model
        .set_attributes(
            id,
            [
                (2, Value::Text("Exterior wall".into())),
                (0, Value::Text("new-guid".into())),
            ],
        )
        .unwrap();

    assert_eq!(
        previous,
        vec![
            Value::Text("Interior wall".into()),
            Value::Text("guid".into()),
        ]
    );
    let entity = model.get(id).unwrap();
    assert_eq!(entity.text(0), Some("new-guid"));
    assert_eq!(entity.text(2), Some("Exterior wall"));
}

#[test]
fn retype_moves_the_id_between_type_index_buckets() {
    let mut model = Model::new();
    let id = model.push(Entity::new("IFCWALL", vec![]));
    assert_eq!(model.ids_of_type("IFCWALL"), &[id]);
    assert_eq!(model.ids_of_type("IFCSLAB"), &[] as &[EntityId]);

    let previous = model.retype(id, "IFCSLAB");

    assert_eq!(previous.as_deref(), Some("IFCWALL"));
    assert_eq!(model.ids_of_type("IFCWALL"), &[] as &[EntityId]);
    assert_eq!(model.ids_of_type("IFCSLAB"), &[id]);
    assert!(model.get(id).unwrap().is_type("IFCSLAB"));
}

#[test]
fn retype_to_the_same_type_is_a_no_op_on_the_index() {
    let mut model = Model::new();
    let a = model.push(Entity::new("IFCWALL", vec![]));
    let b = model.push(Entity::new("IFCWALL", vec![]));

    model.retype(a, "IFCWALL");

    // Both ids must still be present exactly once -- a naive reindex that
    // always removes-then-adds could duplicate `a` in the bucket.
    let mut ids = model.ids_of_type("IFCWALL").to_vec();
    ids.sort_by_key(|id| id.0);
    let mut expected = vec![a, b];
    expected.sort_by_key(|id| id.0);
    assert_eq!(ids, expected);
}

#[test]
fn retype_on_a_missing_id_returns_none() {
    let mut model = Model::new();
    assert_eq!(model.retype(EntityId(42), "IFCWALL"), None);
}

#[test]
fn retype_case_insensitive_relabel_still_finds_the_entity_under_either_case() {
    let mut model = Model::new();
    let id = model.push(Entity::new("IFCWALL", vec![]));

    model.retype(id, "ifcwall");

    // Case-insensitive: relabeling to a differently-cased spelling of the
    // same type must not orphan the id from the index.
    assert_eq!(model.ids_of_type("IFCWALL"), &[id]);
    assert_eq!(model.ids_of_type("ifcwall"), &[id]);
}

#[test]
fn remove_deletes_the_entity_and_its_type_index_entry() {
    let mut model = Model::new();
    let a = model.push(Entity::new("IFCWALL", vec![]));
    let b = model.push(Entity::new("IFCWALL", vec![]));

    let removed = model.remove(a);

    assert!(removed.is_some());
    assert_eq!(removed.unwrap().type_name.as_ref(), "IFCWALL");
    assert_eq!(model.get(a), None);
    assert_eq!(model.len(), 1);
    assert_eq!(model.ids_of_type("IFCWALL"), &[b]);
    assert_eq!(model.ids().collect::<Vec<_>>(), vec![b]);
}

#[test]
fn remove_on_a_missing_id_returns_none_and_touches_nothing() {
    let mut model = Model::new();
    let a = model.push(Entity::new("IFCWALL", vec![]));

    let removed = model.remove(EntityId(999));

    assert_eq!(removed, None);
    assert_eq!(model.len(), 1);
    assert_eq!(model.ids_of_type("IFCWALL"), &[a]);
}

#[test]
fn remove_leaves_referrers_dangling_rather_than_rewriting_them() {
    let mut model = Model::new();
    let point = model.push(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Null, Value::Ref(point)]));

    model.remove(point);

    // Removal is a structural primitive, not a cascading edit: the wall
    // still references the now-missing point, exactly like a hand-edited
    // STEP file would, so `dangling_references` can find it.
    assert_eq!(model.dangling_references(), vec![(wall, point)]);
}

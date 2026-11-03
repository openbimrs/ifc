use ifc_model::{Edit, Entity, EntityId, Model, Value};

use super::Overlay;

fn entity(type_name: &str) -> Entity {
    Entity::new(type_name, vec![Value::Null, Value::Null])
}

/// Walls #1 and #2, a slab #3.
fn model() -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), entity("IFCWALL"));
    model.insert(EntityId(2), entity("IFCWALL"));
    model.insert(EntityId(3), entity("IFCSLAB"));
    model
}

fn retypes(overlay: Overlay<'_>) -> Vec<Edit> {
    overlay
        .into_transaction()
        .edits()
        .iter()
        .filter(|edit| matches!(edit, Edit::Retype { .. }))
        .cloned()
        .collect()
}

#[test]
fn a_retyped_base_entity_moves_buckets_and_stages_one_retype() {
    let mut committed = model();
    let mut overlay = Overlay::new(&committed);
    let created = overlay.allocate();
    overlay.create(created, entity("IFCSLAB"));
    overlay.replace(EntityId(1), entity("IFCSLAB"));

    assert_eq!(overlay.ids_of_type("IFCWALL"), vec![EntityId(2)]);
    let predicted = overlay.ids_of_type("IFCSLAB");
    assert_eq!(predicted, vec![EntityId(3), created, EntityId(1)]);

    let tx = overlay.into_transaction();
    let retypes: Vec<_> = tx
        .edits()
        .iter()
        .filter(|edit| matches!(edit, Edit::Retype { .. }))
        .collect();
    assert_eq!(retypes.len(), 1);
    tx.commit(&mut committed).expect("commits");
    assert_eq!(committed.ids_of_type("IFCSLAB"), predicted);
    assert_eq!(committed.ids_of_type("IFCWALL"), [EntityId(2)]);
}

#[test]
fn retyping_back_stages_nothing() {
    let base = model();
    let mut overlay = Overlay::new(&base);
    overlay.replace(EntityId(1), entity("IFCSLAB"));
    overlay.replace(EntityId(1), entity("IFCWALL"));
    assert_eq!(
        overlay.ids_of_type("IFCWALL"),
        vec![EntityId(1), EntityId(2)]
    );
    assert!(overlay.ids_of_type("IFCSLAB").contains(&EntityId(3)));
    assert_eq!(overlay.ids_of_type("IFCSLAB").len(), 1);
    assert!(retypes(overlay).is_empty());
}

#[test]
fn removing_a_retyped_entity_stages_only_the_removal() {
    let base = model();
    let mut overlay = Overlay::new(&base);
    overlay.replace(EntityId(1), entity("IFCSLAB"));
    overlay.remove(EntityId(1));
    assert_eq!(overlay.ids_of_type("IFCSLAB"), vec![EntityId(3)]);
    let tx = overlay.into_transaction();
    assert!(tx
        .edits()
        .iter()
        .all(|edit| !matches!(edit, Edit::Retype { .. })));
    assert!(tx
        .edits()
        .iter()
        .any(|edit| matches!(edit, Edit::Remove { id } if *id == EntityId(1))));
}

#[test]
fn a_case_only_difference_is_no_retype() {
    let base = model();
    let mut overlay = Overlay::new(&base);
    overlay.replace(EntityId(1), entity("IfcWall"));
    assert_eq!(
        overlay.ids_of_type("IFCWALL"),
        vec![EntityId(1), EntityId(2)]
    );
    assert!(retypes(overlay).is_empty());
}

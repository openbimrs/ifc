//! Sweep the newly authored `IfcReference`.
//!
//! `IfcReference` is self-recursive: `InnerReference` points at
//! another `IfcReference`, so a path walks into a nested value.

use ifc_constraint::{create_reference, ReferenceDraft};
use ifc_model::{Model, Transaction, Value};

/// A reference stages its five slots in order.
#[test]
fn a_reference_stages_its_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_reference(
        &mut tx,
        &model,
        ReferenceDraft::new()
            .type_identifier("IfcWall")
            .attribute_identifier("Name")
            .instance_name("Wall 1")
            .list_positions(&[]),
    )
    .expect("reference");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCREFERENCE");
    assert_eq!(staged.attributes.len(), 5);
    assert_eq!(
        staged.attributes[4],
        Value::Null,
        "an absent InnerReference stays null",
    );
}

/// `InnerReference` must itself be an `IfcReference`.
///
/// The slot is self-typed, so a reference to anything else
/// makes the path unwalkable.
#[test]
fn an_inner_reference_must_be_a_reference() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let inner = create_reference(
        &mut tx,
        &model,
        ReferenceDraft::new()
            .type_identifier("IfcWall")
            .list_positions(&[]),
    )
    .expect("inner");

    let outer = create_reference(
        &mut tx,
        &model,
        ReferenceDraft::new()
            .attribute_identifier("HasProperties")
            .list_positions(&[2])
            .inner_reference(inner),
    )
    .expect("outer");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(outer).expect("staged");
    assert_eq!(staged.attributes[4], Value::Ref(inner));
    assert_eq!(
        staged.attributes[3],
        Value::List(vec![Value::Integer(2)]),
        "ListPositions is a LIST [1:?] OF IfcInteger",
    );
}

/// A non-reference inner target is refused.
#[test]
fn a_non_reference_inner_target_is_refused() {
    use ifc_model::Entity;
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let wall = tx.create(Entity::new("IFCWALL", vec![Value::Null; 8]));
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    assert!(
        create_reference(
            &mut tx,
            &model,
            ReferenceDraft::new()
                .list_positions(&[])
                .inner_reference(wall),
        )
        .is_err(),
        "accepted a wall as an InnerReference",
    );
}

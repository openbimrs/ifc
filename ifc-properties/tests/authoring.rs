//! `ifc-properties` authoring: staged quantity edits (PROP-EDIT).

mod common;

use common::fixture;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_quantity_to_set, create_quantity, quantity_set, quantity_sets, set_description,
    set_quantity_value, PropertyError, Quantity, QuantityKind,
};

// ---- PROP-EDIT -----------------------------------------------------------

/// A staged quantity update keeps the declared measure type.
///
/// Writing a bare real would produce a file that parses and has lost the
/// statement of what the number means.
#[test]
fn a_quantity_update_preserves_its_measure() {
    let mut model = fixture();
    let set = quantity_sets(&model).0.into_iter().next().expect("a set");
    let area = set
        .quantities
        .iter()
        .find_map(|q| match q {
            Quantity::Simple { id, kind, .. } if *kind == QuantityKind::Area => Some(*id),
            _ => None,
        })
        .expect("an area quantity");

    let mut tx = Transaction::new(&model);
    set_quantity_value(&mut tx, &model, area, 42.5).expect("area is a simple quantity");
    tx.commit(&mut model).expect("no conflicts");

    let stored = model.get(area).unwrap().attribute(3).unwrap();
    match stored {
        Value::Typed { type_name, value } => {
            assert_eq!(&**type_name, "IfcAreaMeasure", "the measure survives");
            assert_eq!(value.as_f64(), Some(42.5));
        }
        other => panic!("expected a typed measure, got {other:?}"),
    }
}

/// A count is written as an integer, because IfcCountMeasure is one.
#[test]
fn a_count_is_written_as_an_integer() {
    let mut model = Model::new();
    let count = model.push(Entity::new(
        "IFCQUANTITYCOUNT",
        vec![
            Value::Text("Number".into()),
            Value::Null,
            Value::Null,
            Value::Typed {
                type_name: "IfcCountMeasure".into(),
                value: Box::new(Value::Integer(1)),
            },
        ],
    ));

    let mut tx = Transaction::new(&model);
    set_quantity_value(&mut tx, &model, count, 7.0).expect("a count quantity");
    tx.commit(&mut model).expect("no conflicts");

    let stored = model.get(count).unwrap().attribute(3).unwrap();
    let Value::Typed { value, .. } = stored else {
        panic!("expected typed");
    };
    assert_eq!(
        **value,
        Value::Integer(7),
        "a count is an integer, not a real"
    );
}

/// Editing something that is not a quantity is refused before staging.
#[test]
fn editing_a_non_quantity_is_refused() {
    let model = fixture();
    let wall = model
        .ids_of_type("IFCWALL")
        .first()
        .copied()
        .expect("a wall");

    let mut tx = Transaction::new(&model);
    let result = set_quantity_value(&mut tx, &model, wall, 1.0);

    assert!(
        matches!(result, Err(PropertyError::NotAQuantity { .. })),
        "got {result:?}"
    );
    assert!(tx.is_empty(), "a refused edit stages nothing");
}

/// A whole takeoff lands together or not at all.
///
/// The second update names a missing entity, so the transaction is refused
/// and the first update -- which was perfectly valid -- does not land either.
#[test]
fn a_failed_takeoff_leaves_every_quantity_unchanged() {
    let mut model = fixture();
    let set = quantity_sets(&model).0.into_iter().next().expect("a set");
    let area = set
        .quantities
        .iter()
        .find_map(|q| match q {
            Quantity::Simple { id, kind, .. } if *kind == QuantityKind::Area => Some(*id),
            _ => None,
        })
        .expect("an area quantity");
    let before = model.get(area).unwrap().attribute(3).unwrap().clone();

    let mut tx = Transaction::new(&model);
    set_quantity_value(&mut tx, &model, area, 99.0).expect("valid");
    // Reference an entity that will not exist: the batch must be refused.
    tx.set_attribute(area, 2, Value::Ref(EntityId(31337)));

    assert!(tx.commit(&mut model).is_err(), "the batch is refused");
    assert_eq!(
        model.get(area).unwrap().attribute(3).unwrap(),
        &before,
        "the valid edit did not land either"
    );
}

/// A new quantity can be created and attached in one transaction.
#[test]
fn a_new_quantity_attaches_to_its_set_atomically() {
    let mut model = fixture();
    let set = quantity_sets(&model).0.into_iter().next().expect("a set");
    let before = quantity_set(&model, set.id)
        .expect("readable")
        .0
        .quantities
        .len();

    let mut tx = Transaction::new(&model);
    let weight = create_quantity(&mut tx, QuantityKind::Weight, "GrossWeight", 12.0);
    add_quantity_to_set(&mut tx, &model, set.id, &[weight]).expect("a real set");
    tx.commit(&mut model).expect("the new quantity resolves");

    let after = quantity_set(&model, set.id).expect("readable").0;
    assert_eq!(after.quantities.len(), before + 1);
    assert!(
        after.quantities.iter().any(|q| matches!(
            q,
            Quantity::Simple {
                kind: QuantityKind::Weight,
                ..
            }
        )),
        "the weight is readable through the normal reader"
    );
}

/// Clearing a description writes STEP's `$`, not an empty string.
#[test]
fn a_cleared_description_is_unset_not_blank() {
    let mut model = fixture();
    let set = quantity_sets(&model).0.into_iter().next().expect("a set");
    let Quantity::Simple { id, .. } = set.quantities[0] else {
        panic!("expected a simple quantity");
    };

    let mut tx = Transaction::new(&model);
    set_description(&mut tx, &model, id, Some("measured on site")).expect("exists");
    tx.commit(&mut model).expect("no conflicts");
    assert_eq!(
        model.get(id).unwrap().attribute(1).unwrap().as_text(),
        Some("measured on site")
    );

    let mut tx = Transaction::new(&model);
    set_description(&mut tx, &model, id, None).expect("exists");
    tx.commit(&mut model).expect("no conflicts");
    assert_eq!(
        model.get(id).unwrap().attribute(1).unwrap(),
        &Value::Null,
        "cleared means unset, which survives a round trip as $"
    );
}

//! `IfcQuantityNumber`: the dimensionless quantity.
//!
//! IFC4X3 only, and the one quantity that may be negative: it counts
//! or rates rather than measuring a physical extent.

use ifc_cost::mutation::{create_quantity, QuantityDraft, QuantityKind};
use ifc_model::{Model, Transaction};

fn draft(kind: QuantityKind, value: f64) -> QuantityDraft<'static> {
    QuantityDraft {
        kind,
        name: "Rating",
        description: None,
        unit: None,
        value,
        formula: None,
    }
}

/// A number quantity stages its five slots.
#[test]
fn a_number_quantity_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_quantity(&mut tx, &model, draft(QuantityKind::Number, 7.0))
        .expect("number quantity");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCQUANTITYNUMBER");
    assert_eq!(staged.attributes.len(), 5);
}

/// A number may be negative; a physical extent may not.
#[test]
fn only_the_number_quantity_accepts_a_negative() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    create_quantity(&mut tx, &model, draft(QuantityKind::Number, -3.0))
        .expect("a rating or index may be negative");

    for kind in [
        QuantityKind::Length,
        QuantityKind::Area,
        QuantityKind::Volume,
        QuantityKind::Count,
        QuantityKind::Weight,
        QuantityKind::Time,
    ] {
        assert!(
            create_quantity(&mut tx, &model, draft(kind, -3.0)).is_err(),
            "{kind:?} accepted a negative extent",
        );
    }

    // Non-finite is refused for every kind, including Number.
    assert!(
        create_quantity(&mut tx, &model, draft(QuantityKind::Number, f64::NAN)).is_err(),
        "a NaN number was accepted",
    );
}

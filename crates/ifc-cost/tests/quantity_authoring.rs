//! Cost quantity authoring.
//!
//! A cost value is a rate; the quantity is what it multiplies. These tests
//! pin the slot layout the reader depends on -- the measured number sits in
//! slot 3, after Name, Description and Unit inherited through
//! IfcPhysicalQuantity and IfcPhysicalSimpleQuantity -- and the refusals
//! that stop a wrong number reaching a sum.

use ifc_cost::mutation::{
    assign_cost_quantities, create_cost_item, create_cost_value, create_quantity, CostItemDraft,
    CostItemType, CostValueDraft, CostValueKind, QuantityDraft, QuantityKind,
};
use ifc_cost::quantity::CostQuantity;
use ifc_model::{Model, Transaction};

fn draft(kind: QuantityKind, value: f64) -> QuantityDraft<'static> {
    QuantityDraft {
        kind,
        name: "Measured",
        description: None,
        unit: None,
        value,
        formula: None,
    }
}

#[test]
fn an_authored_quantity_reads_back_through_the_reader() {
    // The reader finds the measure by scanning past the unit slot. If the
    // constructor skipped the two inherited IfcPhysicalQuantity slots, the
    // number would sit where Name belongs and value() would miss it.
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let area =
        create_quantity(&mut tx, &model, draft(QuantityKind::Area, 12.5)).expect("area quantity");
    tx.commit(&mut model).expect("commit");
    let entity = model.get(area).expect("stored");
    let view = CostQuantity::new(area, entity);
    assert_eq!(view.name(), Some("Measured"), "Name must be slot 0");
    assert_eq!(view.value(), Some(12.5), "the measure must be readable");
}

#[test]
fn a_negative_or_fractional_measure_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        create_quantity(&mut tx, &model, draft(QuantityKind::Area, -1.0)).is_err(),
        "no physical extent is negative"
    );
    assert!(
        create_quantity(&mut tx, &model, draft(QuantityKind::Count, 2.5)).is_err(),
        "a count is a cardinality, not a fraction"
    );
    assert!(
        create_quantity(&mut tx, &model, draft(QuantityKind::Count, 3.0)).is_ok(),
        "a whole count is fine"
    );
}

#[test]
fn quantities_attach_to_the_cost_item_slot_the_reader_uses() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let item = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Screed"),
            description: None,
            object_type: Some("Screeding"),
            identification: None,
            predefined_type: Some(CostItemType::UserDefined),
            cost_values: &[],
        },
    )
    .expect("cost item");
    let area = create_quantity(&mut tx, &model, draft(QuantityKind::Area, 40.0)).expect("area");
    assign_cost_quantities(&mut tx, &model, item, &[area]).expect("attach");
    tx.commit(&mut model).expect("commit");
    let view = ifc_cost::item::CostItem::new(item, model.get(item).expect("stored"));
    let attached = view.quantity_refs();
    assert_eq!(
        attached,
        vec![area],
        "the quantity must land where the reader looks"
    );
}

#[test]
fn an_empty_or_duplicated_attachment_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let item = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "1aBcDeFgHiJkLmNoPqRsTu",
            name: None,
            description: None,
            object_type: None,
            identification: None,
            predefined_type: None,
            cost_values: &[],
        },
    )
    .expect("cost item");
    let q = create_quantity(&mut tx, &model, draft(QuantityKind::Volume, 2.0)).expect("q");
    assert!(
        assign_cost_quantities(&mut tx, &model, item, &[]).is_err(),
        "an empty attachment measures nothing"
    );
    assert!(
        assign_cost_quantities(&mut tx, &model, item, &[q, q]).is_err(),
        "a duplicated quantity would double the sum"
    );
}
#[test]
fn an_entity_that_is_not_a_quantity_is_refused() {
    // CostQuantities is typed to IfcPhysicalQuantity, so the check accepts
    // any concrete subtype -- but it must stay closed. A cost value is not
    // a quantity, and attaching one would make a rate measure itself.
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let item = create_cost_item(
        &mut tx,
        &model,
        CostItemDraft {
            global_id: "2aBcDeFgHiJkLmNoPqRsTu",
            name: None,
            description: None,
            object_type: None,
            identification: None,
            predefined_type: None,
            cost_values: &[],
        },
    )
    .expect("cost item");
    let not_a_quantity = create_cost_value(
        &mut tx,
        &model,
        CostValueDraft {
            name: None,
            description: None,
            applicable_date: None,
            fixed_until_date: None,
            category: None,
            condition: None,
            kind: CostValueKind::Monetary(10.0),
        },
    )
    .expect("cost value");
    assert!(
        assign_cost_quantities(&mut tx, &model, item, &[not_a_quantity]).is_err(),
        "a cost value must not pass as a physical quantity"
    );
}

//! Authoring `IfcCurrencyRelationship`.
//!
//! `ExchangeRate` is an `IfcPositiveRatioMeasure`: a zero rate would
//! value every converted cost at nothing.

use ifc_cost::{create_currency_relationship, create_monetary_unit};
use ifc_model::{Model, Transaction, Value};

/// A currency relationship stages its seven slots.
#[test]
fn a_currency_relationship_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let eur = create_monetary_unit(&mut tx, "EUR").expect("eur");
    let gbp = create_monetary_unit(&mut tx, "GBP").expect("gbp");

    let id =
        create_currency_relationship(&mut tx, &model, eur, gbp, 0.85, Some("2026-09-22T00:00:00"))
            .expect("currency relationship");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCCURRENCYRELATIONSHIP");
    assert_eq!(staged.attributes.len(), 7);
    assert_eq!(staged.attributes[2], Value::Ref(eur));
    assert_eq!(staged.attributes[3], Value::Ref(gbp));
    assert_eq!(staged.attributes[4], Value::Real(0.85));
}

/// A non-positive rate is refused, and so is a self-conversion.
#[test]
fn an_impossible_rate_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let eur = create_monetary_unit(&mut tx, "EUR").expect("eur");
    let gbp = create_monetary_unit(&mut tx, "GBP").expect("gbp");

    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            create_currency_relationship(&mut tx, &model, eur, gbp, rate, None).is_err(),
            "rate {rate} was accepted",
        );
    }
    assert!(
        create_currency_relationship(&mut tx, &model, eur, eur, 1.0, None).is_err(),
        "a currency was converted to itself",
    );
}

/// A reference that is not a monetary unit is refused.
#[test]
fn a_non_monetary_reference_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let eur = create_monetary_unit(&mut tx, "EUR").expect("eur");
    let stray = tx.create(ifc_model::Entity::new("IFCSIUNIT", vec![Value::Null; 4]));

    assert!(
        create_currency_relationship(&mut tx, &model, eur, stray, 1.0, None).is_err(),
        "an IfcSIUnit was accepted as a monetary unit",
    );
}

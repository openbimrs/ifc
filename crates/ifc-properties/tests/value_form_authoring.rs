//! Every writer of an `IfcValue` slot refuses a bare literal (#215).
//!
//! `IfcValue` is a SELECT in every release, so ISO 10303-21 writes its value
//! as a typed parameter naming the member (§12.1.8). A bare `2.5` written
//! there cannot say whether it is a length or an area, and `ifc-validate`
//! reports it as `type.select.untyped`; the writers refuse it before
//! staging, as `ifc-author` does.

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_measure_with_unit, add_property_bounded_value, add_property_enumerated_value,
    add_property_enumeration, add_property_list_value, add_property_single_value,
    add_property_table_value, PropertyError, PropertyResult, TableValueDraft,
};

fn length(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(Value::Real(value)),
    }
}

/// Runs `write` on a fresh transaction, and requires it to refuse a bare
/// value in `attribute` with nothing staged.
fn refuses_bare(attribute: &str, write: impl FnOnce(&mut Transaction) -> PropertyResult<EntityId>) {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = write(&mut tx).expect_err("a bare value in an IfcValue slot");
    assert!(
        matches!(
            &err,
            PropertyError::ValueForm {
                attribute: found,
                declared: "IfcValue",
                typed_required: true,
                ..
            } if *found == attribute
        ),
        "{attribute}: {err:?}"
    );
    assert!(tx.is_empty(), "{attribute}: nothing is staged");
}

#[test]
fn every_ifc_value_writer_refuses_a_bare_literal() {
    refuses_bare("NominalValue", |tx| {
        add_property_single_value(tx, "Height", None, Some(Value::Real(3.2)), None)
    });
    refuses_bare("EnumerationValues", |tx| {
        add_property_enumerated_value(
            tx,
            "Finish",
            None,
            Some(vec![Value::Text("Painted".into())]),
            None,
        )
    });
    refuses_bare("LowerBoundValue", |tx| {
        add_property_bounded_value(
            tx,
            "Thickness",
            None,
            Some(length(0.4)),
            Some(Value::Real(0.2)),
            None,
            None,
        )
    });
    refuses_bare("SetPointValue", |tx| {
        add_property_bounded_value(
            tx,
            "Thickness",
            None,
            None,
            None,
            Some(Value::Integer(1)),
            None,
        )
    });
    refuses_bare("ListValues", |tx| {
        add_property_list_value(
            tx,
            "Layers",
            None,
            Some(vec![length(0.1), Value::Real(0.3)]),
            None,
        )
    });
    refuses_bare("DefinedValues", |tx| {
        add_property_table_value(
            tx,
            TableValueDraft::new("Deflection")
                .defining(vec![length(0.0)])
                .defined(vec![Value::Real(0.0)]),
        )
    });
    refuses_bare("EnumerationValues", |tx| {
        add_property_enumeration(tx, "Sizes", vec![Value::Integer(1)], None)
    });
    refuses_bare("ValueComponent", |tx| {
        add_measure_with_unit(tx, Value::Real(0.3048), EntityId(1))
    });
}

/// A reference is no `IfcValue` at all: a type error, not a form error.
#[test]
fn a_reference_is_no_ifc_value() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err =
        add_property_single_value(&mut tx, "Height", None, Some(Value::Ref(EntityId(7))), None)
            .expect_err("a reference in an IfcValue slot");
    assert!(
        matches!(
            err,
            PropertyError::AuthoringInvalid {
                attribute: "NominalValue",
                ..
            }
        ),
        "{err:?}"
    );
    assert!(tx.is_empty());
}

/// The typed form, and an unset optional value, are written as before.
#[test]
fn the_typed_form_is_accepted() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    add_property_single_value(&mut tx, "Height", None, Some(length(3.2)), None).expect("typed");
    add_property_single_value(&mut tx, "Unset", None, None, None).expect("unset");
    add_property_list_value(&mut tx, "Layers", None, Some(vec![length(0.1)]), None)
        .expect("typed list");
    add_measure_with_unit(&mut tx, length(0.3048), EntityId(1)).expect("typed measure");
}

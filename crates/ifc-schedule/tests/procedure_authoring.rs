//! Authoring `IfcProcedure`.
//!
//! `HasName` makes `Name` mandatory even though the slot is
//! OPTIONAL: a procedure nothing can name cannot be referred to
//! by the work that must follow it.

use ifc_model::{Model, Transaction, Value};
use ifc_schedule::{create_procedure, ProcedureDraft};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// A nameless procedure is refused even though the slot is optional.
#[test]
fn a_nameless_procedure_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    for name in [None, Some(""), Some("   ")] {
        create_procedure(
            &mut tx,
            ProcedureDraft {
                global_id: GUID,
                name,
                ..ProcedureDraft::default()
            },
        )
        .expect_err("HasName");
    }
    assert!(tx.is_empty(), "nothing staged when the rule fails");
}

/// The attributes land where the schema declares them.
#[test]
fn the_tail_lands_in_the_declared_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let procedure = create_procedure(
        &mut tx,
        ProcedureDraft {
            global_id: GUID,
            name: Some("Commission chiller"),
            identification: Some("PRC-14"),
            long_description: Some("Full commissioning sequence"),
            predefined_type: Some("STARTUP"),
            ..ProcedureDraft::default()
        },
    )
    .expect("procedure");
    tx.commit(&mut model).expect("commit");
    let p = model.get(procedure).expect("procedure");
    assert_eq!(p.attributes.len(), 8, "IfcProcedure arity");
    assert_eq!(
        p.attributes[2],
        Value::Text("Commission chiller".into()),
        "Name"
    );
    assert_eq!(
        p.attributes[5],
        Value::Text("PRC-14".into()),
        "Identification"
    );
    assert_eq!(
        p.attributes[6],
        Value::Text("Full commissioning sequence".into())
    );
    assert_eq!(
        p.attributes[7],
        Value::Enum("STARTUP".into()),
        "PredefinedType"
    );
}

/// `CorrectPredefinedType`: USERDEFINED needs ObjectType.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_procedure(
        &mut tx,
        ProcedureDraft {
            global_id: GUID,
            name: Some("Bespoke purge"),
            predefined_type: Some("USERDEFINED"),
            ..ProcedureDraft::default()
        },
    )
    .expect_err("CorrectPredefinedType");
    create_procedure(
        &mut tx,
        ProcedureDraft {
            global_id: GUID,
            name: Some("Bespoke purge"),
            object_type: Some("Nitrogen purge"),
            predefined_type: Some("USERDEFINED"),
            ..ProcedureDraft::default()
        },
    )
    .expect("named kind is accepted");
    assert_eq!(tx.len(), 1, "only the named draft staged");
}

/// A token outside `IfcProcedureTypeEnum` is refused.
#[test]
fn a_token_outside_the_enum_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_procedure(
        &mut tx,
        ProcedureDraft {
            global_id: GUID,
            name: Some("Commission"),
            predefined_type: Some("COMMISSIONING"),
            ..ProcedureDraft::default()
        },
    )
    .expect_err("COMMISSIONING is not a declared token");
}

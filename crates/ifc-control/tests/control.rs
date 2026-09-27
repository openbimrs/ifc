//! Authoring the four `IfcControl` subtypes.
//!
//! The four share slots 0-5 and diverge after. The tests that matter
//! are the ones that prove the divergence is honoured rather than
//! papered over: a `Status` on a performance history and a
//! `LifeCyclePhase` on a permit are both refusals.

use ifc_control::{create_control, ControlDraft, ControlError, ControlKind};
use ifc_model::{Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};

const GUID: &str = "0RSPnzHdf5hAmvCJDbRDzy";

fn named(name: &str) -> ControlDraft<'_> {
    ControlDraft {
        name: Some(name),
        ..ControlDraft::default()
    }
}

fn invalid(err: &ControlError) -> bool {
    matches!(err, ControlError::AuthoringInvalid { .. })
}

/// Each control lands its attributes in the slots the schema declares.
#[test]
fn attributes_land_in_the_declared_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let draft = ControlDraft {
        name: Some("Demolition consent"),
        description: Some("Phase 1"),
        identification: Some("PRM-2026-014"),
        status: Some("GRANTED"),
        long_description: Some("Covers the east wing only."),
        ..ControlDraft::default()
    };
    let permit = create_control(
        &mut tx,
        ifc4(),
        ControlKind::Permit,
        GUID,
        Some("BUILDING"),
        draft,
    )
    .expect("permit");

    tx.commit(&mut model).expect("commit");
    let entity = model.get(permit).expect("entity");
    assert_eq!(entity.type_name.as_ref(), "IFCPERMIT");
    assert_eq!(entity.attributes.len(), 9, "declared arity");
    assert_eq!(entity.attributes[0], Value::Text(GUID.into()));
    assert_eq!(
        entity.attributes[2],
        Value::Text("Demolition consent".into())
    );
    assert_eq!(entity.attributes[5], Value::Text("PRM-2026-014".into()));
    assert_eq!(entity.attributes[6], Value::Enum("BUILDING".into()));
    assert_eq!(entity.attributes[7], Value::Text("GRANTED".into()));
}

/// `IfcPerformanceHistory` diverges after slot 5 and the writer
/// honours it in both directions.
#[test]
fn performance_history_has_its_own_tail() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let draft = ControlDraft {
        name: Some("Chiller COP"),
        life_cycle_phase: Some("OPERATION"),
        ..ControlDraft::default()
    };
    let history = create_control(
        &mut tx,
        ifc4(),
        ControlKind::PerformanceHistory,
        GUID,
        Some("NOTDEFINED"),
        draft,
    )
    .expect("history");

    // The phase occupies slot 6, where the other controls put nothing,
    // and the predefined type moves to 7.
    tx.commit(&mut model).expect("commit");
    let entity = model.get(history).expect("entity");
    assert_eq!(entity.attributes.len(), 8, "declared arity");
    assert_eq!(entity.attributes[6], Value::Text("OPERATION".into()));
    assert_eq!(entity.attributes[7], Value::Enum("NOTDEFINED".into()));
}

/// The phase is required, and absent it the record says nothing about
/// when the behaviour was observed.
#[test]
fn a_performance_history_without_a_phase_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_control(
        &mut tx,
        ifc4(),
        ControlKind::PerformanceHistory,
        GUID,
        None,
        named("Chiller COP"),
    )
    .expect_err("LifeCyclePhase");
    assert!(invalid(&err), "{err}");
    assert!(tx.is_empty(), "nothing is staged when the rule fails");
}

/// An attribute the entity does not declare is refused, not dropped.
///
/// Dropping it writes a file missing data the caller believes they
/// supplied, which is worse than a refusal because nothing reports it.
#[test]
fn undeclared_attributes_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // A performance history declares neither Status nor LongDescription.
    for draft in [
        ControlDraft {
            name: Some("H"),
            life_cycle_phase: Some("OPERATION"),
            status: Some("OPEN"),
            ..ControlDraft::default()
        },
        ControlDraft {
            name: Some("H"),
            life_cycle_phase: Some("OPERATION"),
            long_description: Some("..."),
            ..ControlDraft::default()
        },
    ] {
        let err = create_control(
            &mut tx,
            ifc4(),
            ControlKind::PerformanceHistory,
            GUID,
            None,
            draft,
        )
        .expect_err("not declared");
        assert!(invalid(&err), "{err}");
    }

    // The other three declare no LifeCyclePhase.
    for kind in [
        ControlKind::Permit,
        ControlKind::ProjectOrder,
        ControlKind::ActionRequest,
    ] {
        let draft = ControlDraft {
            name: Some("C"),
            life_cycle_phase: Some("OPERATION"),
            ..ControlDraft::default()
        };
        let err =
            create_control(&mut tx, ifc4(), kind, GUID, None, draft).expect_err("not declared");
        assert!(invalid(&err), "{err}");
    }
    assert!(tx.is_empty(), "nothing is staged when the rule fails");
}

/// A token from a sibling control's enum is refused.
///
/// The four enums are disjoint apart from the two shared fallbacks, so
/// a writer keyed on the wrong one files a permit as a phone call.
#[test]
fn a_token_from_another_control_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    // WORKORDER is IfcProjectOrder's; PHONE is IfcActionRequest's.
    for (kind, token) in [
        (ControlKind::Permit, "WORKORDER"),
        (ControlKind::Permit, "PHONE"),
        (ControlKind::ActionRequest, "BUILDING"),
        (ControlKind::ProjectOrder, "ACCESS"),
        (ControlKind::PerformanceHistory, "WORKORDER"),
    ] {
        let err = create_control(&mut tx, ifc4(), kind, GUID, Some(token), named("C"))
            .expect_err("foreign token");
        assert!(invalid(&err), "{err}");
    }

    // Each entity's own tokens are accepted.
    for (kind, token) in [
        (ControlKind::Permit, "BUILDING"),
        (ControlKind::ProjectOrder, "WORKORDER"),
        (ControlKind::ActionRequest, "PHONE"),
        (ControlKind::PerformanceHistory, "NOTDEFINED"),
    ] {
        let draft = ControlDraft {
            name: Some("C"),
            life_cycle_phase: if kind == ControlKind::PerformanceHistory {
                Some("OPERATION")
            } else {
                None
            },
            ..ControlDraft::default()
        };
        create_control(&mut tx, ifc4(), kind, GUID, Some(token), draft).expect("own token");
    }
}

/// `USERDEFINED` names a kind the enum has no token for, and
/// `ObjectType` is where that name goes.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let err = create_control(
        &mut tx,
        ifc4(),
        ControlKind::Permit,
        GUID,
        Some("USERDEFINED"),
        named("Consent"),
    )
    .expect_err("USERDEFINED");
    assert!(invalid(&err), "{err}");

    // Blank is not a name either.
    let blank = ControlDraft {
        name: Some("Consent"),
        object_type: Some("   "),
        ..ControlDraft::default()
    };
    create_control(
        &mut tx,
        ifc4(),
        ControlKind::Permit,
        GUID,
        Some("USERDEFINED"),
        blank,
    )
    .expect_err("blank ObjectType");
    assert!(tx.is_empty(), "nothing is staged when the rule fails");

    // With the name supplied it stages.
    let ok = ControlDraft {
        name: Some("Consent"),
        object_type: Some("Heritage consent"),
        ..ControlDraft::default()
    };
    create_control(
        &mut tx,
        ifc4(),
        ControlKind::Permit,
        GUID,
        Some("USERDEFINED"),
        ok,
    )
    .expect("named USERDEFINED");
}

/// A malformed GlobalId and a blank name are both refused.
#[test]
fn identity_is_validated() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_control(
        &mut tx,
        ifc4(),
        ControlKind::Permit,
        "not-a-guid",
        None,
        named("Consent"),
    )
    .expect_err("GlobalId");

    for name in ["", "   "] {
        create_control(
            &mut tx,
            ifc4(),
            ControlKind::Permit,
            GUID,
            None,
            named(name),
        )
        .expect_err("Name");
    }
    assert!(tx.is_empty(), "nothing is staged when the rule fails");
}

/// The hardcoded enums and slot assumptions match both shipped
/// schemas.
///
/// Arity is read from the schema at write time, so this test exists to
/// catch the reverse: a schema that stops declaring what the writer
/// assumes about slot meaning.
#[test]
fn the_writer_agrees_with_both_schemas() {
    for schema in [ifc4(), ifc4x3()] {
        for (kind, arity) in [
            (ControlKind::Permit, 9),
            (ControlKind::ProjectOrder, 9),
            (ControlKind::ActionRequest, 9),
            (ControlKind::PerformanceHistory, 8),
        ] {
            let name = kind.type_name();
            let declared = schema.attributes(name);
            assert_eq!(
                declared.len(),
                arity,
                "{name} arity drifted in {}",
                schema.name()
            );
            // Slot 5 is Identification on every control.
            assert_eq!(
                declared[5].name.to_ascii_uppercase(),
                "IDENTIFICATION",
                "{name} slot 5"
            );
            // Every declared token is one the writer accepts.
            let mut model = Model::default();
            let mut tx = Transaction::new(&model);
            for token in kind.members() {
                let draft = ControlDraft {
                    name: Some("C"),
                    object_type: Some("named"),
                    life_cycle_phase: if kind == ControlKind::PerformanceHistory {
                        Some("OPERATION")
                    } else {
                        None
                    },
                    ..ControlDraft::default()
                };
                create_control(&mut tx, schema, kind, GUID, Some(token), draft)
                    .expect("declared token");
            }
            tx.commit(&mut model).expect("commit");
        }
    }
}

//! What a release cannot hold is refused with a typed error and nothing is
//! staged (#202). IFC2X3 facts from `IFC2X3_TC1.exp`: `IfcRelSequence`
//! declares `TimeLag : IfcTimeMeasure` and `SequenceType : IfcSequenceEnum`
//! (without `USERDEFINED`), both required; `IfcWorkControl` declares
//! `CreationDate : IfcDateTimeSelect`; `IfcEvent` and `IfcWorkCalendar` are
//! not declared; `IfcProcedure.WR4` ties `USERDEFINED` to
//! `UserDefinedProcedureType`.

mod common;

use common::{base, OWNER, WALL};
use ifc_model::{EntityId, Transaction};
use ifc_schedule::error::ScheduleAuthoringError as E;
use ifc_schedule::{
    create_event_with_owner_history, create_procedure_with_owner_history,
    create_sequence_with_owner_history, create_task_with_owner_history,
    create_work_calendar_with_owner_history, create_work_control_with_owner_history,
    nest_tasks_with_owner_history, EventDraft, ProcedureDraft, SchemaVersion, TaskDraft,
    WorkControlDraft, WorkControlKind,
};

const GUID: &str = "0YvctVUKr0kugbFTf53O08";
const V: SchemaVersion = SchemaVersion::Ifc2x3;

fn refused(tx: &Transaction, result: Result<EntityId, E>) -> E {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

fn task() -> TaskDraft<'static> {
    TaskDraft {
        global_id: GUID,
        name: Some("Slab"),
        identification: Some("T-1"),
        ..TaskDraft::default()
    }
}

#[test]
fn ifc2x3_refuses_task_values_it_lacks() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let not_in = |attribute| E::AuthoringNotInSchema {
        entity: "IFCTASK",
        attribute,
        schema: V,
    };
    for (draft, attribute) in [
        (
            TaskDraft {
                long_description: Some("Long"),
                ..task()
            },
            "LongDescription",
        ),
        (
            TaskDraft {
                task_time: Some(EntityId(3)),
                ..task()
            },
            "TaskTime",
        ),
        (
            TaskDraft {
                predefined_type: Some("CONSTRUCTION"),
                ..task()
            },
            "PredefinedType",
        ),
    ] {
        let result = create_task_with_owner_history(&mut tx, &model, draft, OWNER);
        assert_eq!(refused(&tx, result), not_in(attribute));
    }
    let anonymous = TaskDraft {
        identification: None,
        ..task()
    };
    let result = create_task_with_owner_history(&mut tx, &model, anonymous, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCTASK",
            attribute: "TaskId",
            schema: V,
        }
    );
}

#[test]
fn ifc2x3_refuses_sequences_work_controls_and_absent_entities() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let (a, b) = (EntityId(30), EntityId(31));
    let sequence = |tx: &mut Transaction, kind, lag| {
        create_sequence_with_owner_history(tx, &model, GUID, a, b, kind, lag, OWNER)
    };
    let result = sequence(&mut tx, Some("FINISH_START"), Some(EntityId(32)));
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCRELSEQUENCE",
            attribute: "TimeLag",
            declared: "IfcTimeMeasure",
            schema: V,
        }
    );
    let result = sequence(&mut tx, Some("FINISH_START"), None);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCRELSEQUENCE",
            attribute: "TimeLag",
            schema: V,
        }
    );
    let result = sequence(&mut tx, Some("USERDEFINED"), None);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCRELSEQUENCE",
            attribute: "SequenceType",
            declared: "IfcSequenceEnum",
            schema: V,
        }
    );

    let control = WorkControlDraft {
        global_id: GUID,
        identification: Some("WS-1"),
        creation_date: "2026-09-28T00:00:00",
        start_time: "2026-10-01T08:00:00",
        ..WorkControlDraft::default()
    };
    let result = create_work_control_with_owner_history(
        &mut tx,
        &model,
        WorkControlKind::Schedule,
        control,
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCWORKSCHEDULE",
            attribute: "CreationDate",
            declared: "IfcDateTimeSelect",
            schema: V,
        }
    );

    let result = create_work_calendar_with_owner_history(
        &mut tx,
        &model,
        GUID,
        None,
        &[a],
        &[],
        None,
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        E::EntityNotInSchema {
            entity: "IFCWORKCALENDAR",
            schema: V,
        }
    );
    let event = EventDraft {
        global_id: GUID,
        ..EventDraft::default()
    };
    let result = create_event_with_owner_history(&mut tx, &model, event, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::EntityNotInSchema {
            entity: "IFCEVENT",
            schema: V,
        }
    );
}

#[test]
fn ifc2x3_procedures_need_their_type_and_no_userdefined() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let draft = ProcedureDraft {
        global_id: GUID,
        name: Some("Calibrate"),
        identification: Some("P-1"),
        ..ProcedureDraft::default()
    };
    let result = create_procedure_with_owner_history(&mut tx, &model, draft, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCPROCEDURE",
            attribute: "ProcedureType",
            schema: V,
        }
    );
    let user_defined = ProcedureDraft {
        predefined_type: Some("USERDEFINED"),
        object_type: Some("Flush"),
        ..draft
    };
    let result = create_procedure_with_owner_history(&mut tx, &model, user_defined, OWNER);
    assert!(matches!(
        refused(&tx, result),
        E::InvalidValue {
            attribute: "PredefinedType",
            ..
        }
    ));
}

/// A token IFC4X3 added to `IfcTaskTypeEnum` is refused in IFC4 and
/// accepted in IFC4X3: the enumeration comes from the bound table.
#[test]
fn enumerations_come_from_the_bound_release() {
    let draft = TaskDraft {
        predefined_type: Some("ADJUSTMENT"),
        ..task()
    };
    let ifc4 = base("IFC4", SchemaVersion::Ifc4);
    let mut tx = Transaction::new(&ifc4);
    let result = create_task_with_owner_history(&mut tx, &ifc4, draft, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCTASK",
            attribute: "PredefinedType",
            declared: "IfcTaskTypeEnum",
            schema: SchemaVersion::Ifc4,
        }
    );
    let ifc4x3 = base("IFC4X3_ADD2", SchemaVersion::Ifc4x3);
    let mut tx = Transaction::new(&ifc4x3);
    create_task_with_owner_history(&mut tx, &ifc4x3, draft, OWNER).expect("IFC4X3 token");
}

/// The owner history must exist and be an `IfcOwnerHistory`; the header
/// must bind exactly one known release.
#[test]
fn owner_history_and_binding_are_checked() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let result = create_task_with_owner_history(&mut tx, &model, task(), WALL);
    assert_eq!(
        refused(&tx, result),
        E::WrongReferenceType {
            entity: "IFCTASK",
            attribute: "OwnerHistory",
            target: WALL,
            actual: "IFCWALL".into(),
            expected: "IFCOWNERHISTORY",
        }
    );
    let missing = EntityId(99);
    let result = nest_tasks_with_owner_history(&mut tx, &model, GUID, WALL, &[OWNER], missing);
    assert_eq!(
        refused(&tx, result),
        E::MissingReference {
            entity: "IFCRELNESTS",
            attribute: "OwnerHistory",
            target: missing,
        }
    );
    let mut several = base("IFC2X3", V);
    several.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    let result = create_task_with_owner_history(&mut tx, &several, task(), OWNER);
    assert_eq!(refused(&tx, result), E::MultipleSchemas { schemas: 2 });
    let mut unknown = base("IFC2X3", V);
    unknown.header_mut().schema = vec!["IFC5".into()];
    let result = create_task_with_owner_history(&mut tx, &unknown, task(), OWNER);
    assert_eq!(
        refused(&tx, result),
        E::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );
}

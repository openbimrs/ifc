//! What a release cannot hold is refused with a typed error and nothing is
//! staged (#202). IFC2X3 facts from `IFC2X3_TC1.exp`: `IfcRelSequence`
//! declares `TimeLag : IfcTimeMeasure` and `SequenceType : IfcSequenceEnum`
//! (without `USERDEFINED`), both required; `IfcWorkControl` declares
//! `CreationDate : IfcDateTimeSelect`; `IfcEvent` and `IfcWorkCalendar` are
//! not declared; `IfcProcedure.WR4` ties `USERDEFINED` to
//! `UserDefinedProcedureType`. The forms #214 added are refused in the
//! release that does not declare them, and a record form the schema's rules
//! (`IfcValidCalendarDate`, `IfcValidTime`, the component ranges) refuse is
//! refused before anything is staged.

mod common;

use common::{base, OWNER, WALL};
use ifc_model::{EntityId, Transaction};
use ifc_schedule::error::ScheduleAuthoringError as E;
use ifc_schedule::{
    create_event_with_owner_history, create_procedure, create_procedure_with_owner_history,
    create_sequence_with_owner_history, create_task_with_owner_history,
    create_work_calendar_with_owner_history, create_work_control,
    create_work_control_with_owner_history, nest_tasks_with_owner_history, CalendarDate,
    DateTimeValue, EventDraft, LocalTime, ProcedureDraft, SchemaVersion, TaskDraft, TimeLag,
    WorkControlDraft, WorkControlKind,
};

const GUID: &str = "0YvctVUKr0kugbFTf53O08";
const V: SchemaVersion = SchemaVersion::Ifc2x3;

fn refused(tx: &Transaction, result: Result<EntityId, E>) -> E {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

fn task() -> TaskDraft<'static> {
    TaskDraft::new(GUID).name("Slab").identification("T-1")
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
        (task().long_description("Long"), "LongDescription"),
        (task().task_time(EntityId(3)), "TaskTime"),
        (task().predefined_type("CONSTRUCTION"), "PredefinedType"),
    ] {
        let result = create_task_with_owner_history(&mut tx, &model, draft, OWNER);
        assert_eq!(refused(&tx, result), not_in(attribute));
    }
    let mut anonymous = task();
    anonymous.identification = None;
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
fn ifc2x3_refuses_other_forms_and_absent_entities() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let (a, b) = (EntityId(30), EntityId(31));
    let sequence = |tx: &mut Transaction, kind, lag| {
        create_sequence_with_owner_history(tx, &model, GUID, a, b, kind, lag, OWNER)
    };
    let lag_time = Some(TimeLag::LagTime(EntityId(32)));
    let result = sequence(&mut tx, Some("FINISH_START"), lag_time);
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
    let seconds = Some(TimeLag::Seconds(0.0));
    let result = sequence(&mut tx, None, seconds);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCRELSEQUENCE",
            attribute: "SequenceType",
            schema: V,
        }
    );
    let result = sequence(
        &mut tx,
        Some("FINISH_START"),
        Some(TimeLag::Seconds(f64::NAN)),
    );
    assert!(matches!(
        refused(&tx, result),
        E::InvalidValue {
            attribute: "TimeLag",
            ..
        }
    ));
    let result = sequence(&mut tx, Some("USERDEFINED"), seconds);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCRELSEQUENCE",
            attribute: "SequenceType",
            declared: "IfcSequenceEnum",
            schema: V,
        }
    );

    let control = WorkControlDraft::new(GUID, "2026-09-28T00:00:00", "2026-10-01T08:00:00")
        .identification("WS-1");
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
    let event = EventDraft::new(GUID);
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
fn ifc2x3_procedures_need_their_type_and_a_userdefined_label() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let draft = ProcedureDraft::new(GUID)
        .name("Calibrate")
        .identification("P-1");
    let result = create_procedure_with_owner_history(&mut tx, &model, draft, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCPROCEDURE",
            attribute: "ProcedureType",
            schema: V,
        }
    );
    let user_defined = draft.predefined_type("USERDEFINED").object_type("Flush");
    for label in [None, Some(""), Some("  ")] {
        let mut unlabelled = user_defined;
        unlabelled.user_defined_procedure_type = label;
        let result = create_procedure_with_owner_history(&mut tx, &model, unlabelled, OWNER);
        assert!(matches!(
            refused(&tx, result),
            E::InvalidValue {
                attribute: "UserDefinedProcedureType",
                ..
            }
        ));
    }
    let labelled = user_defined.user_defined_procedure_type("Flushing");
    create_procedure_with_owner_history(&mut tx, &model, labelled, OWNER).expect("WR4 holds");
}

/// IFC4 and IFC4X3 declare neither the IFC2X3 record forms nor an
/// `IfcTimeMeasure` lag nor `UserDefinedProcedureType`.
#[test]
fn ifc4_and_ifc4x3_refuse_the_ifc2x3_forms() {
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let (a, b) = (EntityId(30), EntityId(31));
        let seconds = Some(TimeLag::Seconds(3600.0));
        let result = create_sequence_with_owner_history(
            &mut tx,
            &model,
            GUID,
            a,
            b,
            Some("FINISH_START"),
            seconds,
            OWNER,
        );
        assert_eq!(
            refused(&tx, result),
            E::AuthoringValueType {
                entity: "IFCRELSEQUENCE",
                attribute: "TimeLag",
                declared: "IfcLagTime",
                schema: version,
            }
        );
        let date = CalendarDate::new(2026, 9, 28);
        let control = WorkControlDraft::new(GUID, date, "2026-10-01T08:00:00");
        let result = create_work_control_with_owner_history(
            &mut tx,
            &model,
            WorkControlKind::Plan,
            control,
            OWNER,
        );
        assert_eq!(
            refused(&tx, result),
            E::AuthoringValueType {
                entity: "IFCWORKPLAN",
                attribute: "CreationDate",
                declared: "IfcDateTime",
                schema: version,
            }
        );
        let procedure = ProcedureDraft::new(GUID)
            .name("Flush")
            .user_defined_procedure_type("Flushing");
        let result = create_procedure_with_owner_history(&mut tx, &model, procedure, OWNER);
        assert_eq!(
            refused(&tx, result),
            E::AuthoringNotInSchema {
                entity: "IFCPROCEDURE",
                attribute: "UserDefinedProcedureType",
                schema: version,
            }
        );
    }
}

/// A record form is checked against the IFC2X3 rules before anything,
/// the owning record included, is staged.
#[test]
fn ifc2x3_date_records_follow_the_schema_rules() {
    let model = base("IFC2X3", V);
    let mut tx = Transaction::new(&model);
    let cases: [(DateTimeValue<'static>, &str); 8] = [
        (CalendarDate::new(2026, 13, 1).into(), "MonthComponent"),
        (CalendarDate::new(2026, 0, 1).into(), "MonthComponent"),
        (CalendarDate::new(2026, 4, 31).into(), "DayComponent"),
        (CalendarDate::new(2025, 2, 29).into(), "DayComponent"),
        (LocalTime::new(24).into(), "HourComponent"),
        (LocalTime::new(8).minute(60).into(), "MinuteComponent"),
        (
            LocalTime::new(8).minute(0).second(60.0).into(),
            "SecondComponent",
        ),
        (LocalTime::new(8).second(1.0).into(), "MinuteComponent"),
    ];
    for (date, attribute) in cases {
        let control = WorkControlDraft::new(GUID, CalendarDate::new(2024, 2, 29), date)
            .identification("WS-1");
        let result = create_work_control_with_owner_history(
            &mut tx,
            &model,
            WorkControlKind::Schedule,
            control,
            OWNER,
        );
        match refused(&tx, result) {
            E::InvalidValue { attribute: a, .. } => assert_eq!(a, attribute, "{date:?}"),
            other => panic!("{date:?}: {other:?}"),
        }
    }
    // A refusal of the work control itself stages none of its dates.
    let control = WorkControlDraft::new(GUID, CalendarDate::new(2026, 9, 28), LocalTime::new(8));
    let result = create_work_control_with_owner_history(
        &mut tx,
        &model,
        WorkControlKind::Schedule,
        control,
        OWNER,
    );
    assert!(matches!(
        refused(&tx, result),
        E::AuthoringRequired {
            attribute: "Identifier",
            ..
        }
    ));
    let control = control.identification("WS-1");
    let result = create_work_control_with_owner_history(
        &mut tx,
        &model,
        WorkControlKind::Schedule,
        control,
        WALL,
    );
    assert!(matches!(refused(&tx, result), E::WrongReferenceType { .. }));
}

/// A token IFC4X3 added to `IfcTaskTypeEnum` is refused in IFC4 and
/// accepted in IFC4X3: the enumeration comes from the bound table.
#[test]
fn enumerations_come_from_the_bound_release() {
    let draft = task().predefined_type("ADJUSTMENT");
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

/// The plain writers take no model and write IFC4/IFC4X3 only, so they
/// refuse the IFC2X3 forms outright.
#[test]
fn plain_writers_refuse_the_ifc2x3_forms() {
    let model = base("IFC4", SchemaVersion::Ifc4);
    let mut tx = Transaction::new(&model);
    let dates: [DateTimeValue<'static>; 3] = [
        CalendarDate::new(2026, 9, 28).into(),
        LocalTime::new(8).into(),
        DateTimeValue::DateAndTime(CalendarDate::new(2026, 9, 28), LocalTime::new(8)),
    ];
    for date in dates {
        for control in [
            WorkControlDraft::new(GUID, date, "2026-10-01T08:00:00"),
            WorkControlDraft::new(GUID, "2026-09-28T00:00:00", date),
            WorkControlDraft::new(GUID, "2026-09-28T00:00:00", "2026-10-01T08:00:00")
                .finish_time(date),
        ] {
            let result = create_work_control(&mut tx, WorkControlKind::Plan, control);
            assert!(
                matches!(refused(&tx, result), E::InvalidValue { .. }),
                "{date:?}"
            );
        }
    }
    let procedure = ProcedureDraft::new(GUID)
        .name("Flush")
        .user_defined_procedure_type("Flushing");
    let result = create_procedure(&mut tx, procedure);
    assert!(matches!(
        refused(&tx, result),
        E::InvalidValue {
            attribute: "UserDefinedProcedureType",
            ..
        }
    ));
}

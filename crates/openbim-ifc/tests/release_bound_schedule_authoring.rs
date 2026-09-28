//! Schedule records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#202, #214).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-schedule`, written to STEP, read back with `ifc-step` and through
//! the crate's views, and checked by `ifc-validate` against the declared
//! release's table. No record this test wrote may carry an error finding.
//! IFC2X3 holds tasks, procedures (a `USERDEFINED` one with its
//! `UserDefinedProcedureType`), nesting, control assignments, sequences
//! with an `IfcTimeMeasure` lag, and work plans and schedules whose dates
//! are `IfcCalendarDate`, `IfcLocalTime` and `IfcDateAndTime` records the
//! writer stages (#214). It declares no events or calendars, which are
//! refused (see `ifc-schedule`'s `release_refusals.rs`).

#![cfg(all(
    feature = "validate",
    feature = "schedule",
    feature = "schema",
    feature = "step"
))]

use ifc::schedule::{
    assign_tasks_to_control_with_owner_history, create_event_with_owner_history,
    create_procedure_with_owner_history, create_sequence_with_owner_history,
    create_task_with_owner_history, create_work_calendar_with_owner_history,
    create_work_control_with_owner_history, create_work_time, nest_tasks_with_owner_history,
    sequences, successors_of, tasks_of_schedule, work_plans, work_schedules, CalendarDate,
    DateTimeValue, EventDraft, LocalTime, ProcedureDraft, SequenceType, TaskDraft, TimeLag,
    WorkControlDraft, WorkControlKind,
};
use ifc::schema::{for_version, SchemaVersion};
use ifc::{Codec, Model, StepCodec, Value};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);

const G: [&str; 11] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
    "0YvctVUKr0kugbFTf53O0C",
    "0YvctVUKr0kugbFTf53O0D",
    "0YvctVUKr0kugbFTf53O0E",
    "0YvctVUKr0kugbFTf53O0F",
    "0YvctVUKr0kugbFTf53O0G",
    "0YvctVUKr0kugbFTf53O0H",
    "0YvctVUKr0kugbFTf53O0I",
];

/// Actors and an owner history (`#5`) in `schema`.
fn base(schema: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn task(global_id: &'static str, name: &'static str, ifc4: bool) -> TaskDraft<'static> {
    let mut draft = TaskDraft::new(global_id)
        .name(name)
        .description("Pour")
        .identification("T-1")
        .status("PLANNED")
        .work_method("Pump")
        .priority(10);
    draft.long_description = ifc4.then_some("Long");
    draft.predefined_type = ifc4.then_some("CONSTRUCTION");
    draft
}

/// A work plan and a work schedule with dates in `version`'s form.
fn work_controls(ifc4: bool) -> [WorkControlDraft<'static>; 2] {
    let (created, start, finish): (DateTimeValue<'_>, DateTimeValue<'_>, DateTimeValue<'_>) =
        if ifc4 {
            (
                "2026-09-28T00:00:00".into(),
                "2026-10-01T08:00:00".into(),
                "2026-12-18T17:00:00".into(),
            )
        } else {
            (
                CalendarDate::new(2026, 9, 28).into(),
                DateTimeValue::DateAndTime(
                    CalendarDate::new(2026, 10, 1),
                    LocalTime::new(8).minute(0).second(0.0),
                ),
                LocalTime::new(17).minute(30).into(),
            )
        };
    let mut plan = WorkControlDraft::new(G[9], created, start)
        .name("Masterplan")
        .identification("WP-1")
        .finish_time(finish);
    let mut schedule = WorkControlDraft::new(G[5], created, start)
        .name("Programme")
        .identification("WS-1");
    if ifc4 {
        plan = plan.predefined_type("PLANNED");
        schedule = schedule.predefined_type("PLANNED");
    }
    [plan, schedule]
}

/// Every `IfcRoot` writer `version` can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let mut tx = Transaction::new(model);
    let t1 = create_task_with_owner_history(&mut tx, model, task(G[0], "Slab", ifc4), OWNER)
        .expect("task");
    let t2 = create_task_with_owner_history(&mut tx, model, task(G[1], "Walls", ifc4), OWNER)
        .expect("task");
    let procedure = ProcedureDraft::new(G[2])
        .name("Calibrate")
        .identification("P-1")
        .predefined_type("CALIBRATION");
    let procedure =
        create_procedure_with_owner_history(&mut tx, model, procedure, OWNER).expect("procedure");
    let nest = nest_tasks_with_owner_history(&mut tx, model, G[3], t1, &[t2], OWNER).expect("nest");
    let [plan, schedule] = work_controls(ifc4);
    let plan =
        create_work_control_with_owner_history(&mut tx, model, WorkControlKind::Plan, plan, OWNER)
            .expect("plan");
    let schedule = create_work_control_with_owner_history(
        &mut tx,
        model,
        WorkControlKind::Schedule,
        schedule,
        OWNER,
    )
    .expect("schedule");
    // IFC2X3 requires the lag, as seconds; IFC4 and IFC4X3 leave it unset.
    let lag = (!ifc4).then_some(TimeLag::Seconds(86_400.0));
    let sequence = create_sequence_with_owner_history(
        &mut tx,
        model,
        G[6],
        t1,
        t2,
        Some("FINISH_START"),
        lag,
        OWNER,
    )
    .expect("sequence");
    let mut written = vec![t1, t2, procedure, nest, plan, schedule, sequence];
    if ifc4 {
        let period = create_work_time(&mut tx, Some("Weekdays"), None, None, None).unwrap();
        let calendar = create_work_calendar_with_owner_history(
            &mut tx,
            model,
            G[7],
            Some("Day shift"),
            &[period],
            &[],
            Some("FIRSTSHIFT"),
            OWNER,
        )
        .expect("calendar");
        let event = EventDraft::new(G[8])
            .name("Handover")
            .predefined_type("ENDEVENT")
            .trigger_type("EVENTTIME");
        let event = create_event_with_owner_history(&mut tx, model, event, OWNER).expect("event");
        written.extend([calendar, event]);
    } else {
        // IFC2X3 WR4: USERDEFINED names its kind in UserDefinedProcedureType.
        let flush = ProcedureDraft::new(G[10])
            .name("Flush")
            .identification("P-2")
            .object_type("Flush")
            .predefined_type("USERDEFINED")
            .user_defined_procedure_type("Flushing");
        written.push(
            create_procedure_with_owner_history(&mut tx, model, flush, OWNER).expect("flush"),
        );
    }
    let assigned =
        assign_tasks_to_control_with_owner_history(&mut tx, model, G[4], schedule, &[t1], OWNER)
            .expect("assignment");
    written.push(assigned);
    tx.commit(model).expect("commit");
    // The IFC2X3 date records the work controls reference, and theirs.
    let mut at = 0;
    while at < written.len() {
        let entity = model.get(written[at]).expect("written");
        if matches!(
            &*entity.type_name,
            "IFCWORKPLAN" | "IFCWORKSCHEDULE" | "IFCDATEANDTIME"
        ) {
            let dates: Vec<EntityId> = entity
                .attributes
                .iter()
                .filter_map(|value| match value {
                    Value::Ref(id) if *id != OWNER => Some(*id),
                    _ => None,
                })
                .collect();
            written.extend(dates);
        }
        at += 1;
    }
    written
}

/// Error findings `ifc-validate` reports on `ids`, against `version`.
fn errors(model: &Model, version: SchemaVersion, ids: &[EntityId]) -> Vec<String> {
    let report = ifc_validate::validate(model, for_version(version).expect("bundled"));
    report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .filter(|finding| match &finding.path {
            ifc_validate::Path::Entity(id) | ifc_validate::Path::Attribute { entity: id, .. } => {
                ids.contains(id)
            }
            ifc_validate::Path::File => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_schedule_records_validate_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
        read_back(&back, version, &written);
    }
}

fn names<'m>(
    controls: Vec<ifc::schedule::WorkControl<'m>>,
) -> Vec<(EntityId, Option<&'m str>, Option<&'m str>)> {
    controls
        .iter()
        .map(|control| (control.id(), control.name(), control.identification()))
        .collect()
}

/// The records read back through the crate's views, and in IFC2X3 the
/// date records and the lag hold what was authored.
fn read_back(back: &Model, version: SchemaVersion, written: &[EntityId]) {
    let [t1, t2, _, _, plan, schedule, sequence] = written[..7] else {
        panic!("written");
    };
    assert_eq!(
        names(work_plans(back)),
        vec![(plan, Some("Masterplan"), Some("WP-1"))]
    );
    assert_eq!(
        names(work_schedules(back)),
        vec![(schedule, Some("Programme"), Some("WS-1"))]
    );
    assert_eq!(tasks_of_schedule(back, schedule), vec![t1]);
    assert_eq!(successors_of(back, t1), vec![t2]);
    let link = sequences(back).into_iter().next().expect("sequence");
    assert_eq!(link.id, sequence);
    assert_eq!(link.sequence_type, Some(SequenceType::FinishStart));
    if version != SchemaVersion::Ifc2x3 {
        let schedule = &work_schedules(back)[0];
        assert_eq!(schedule.creation_date(), Some("2026-09-28T00:00:00"));
        assert_eq!(schedule.start_time(), Some("2026-10-01T08:00:00"));
        return;
    }
    let int = Value::Integer;
    let record = |id: EntityId| back.get(id).expect("record");
    let referenced = |id: EntityId, slot: usize| match &record(id).attributes[slot] {
        Value::Ref(target) => record(*target),
        other => panic!("#{} slot {slot}: {other:?}", id.0),
    };
    // IfcWorkControl: CreationDate 6, StartTime 11, FinishTime 12.
    let created = referenced(plan, 6);
    assert_eq!(&*created.type_name, "IFCCALENDARDATE");
    assert_eq!(created.attributes, vec![int(28), int(9), int(2026)]);
    let start = referenced(plan, 11);
    assert_eq!(&*start.type_name, "IFCDATEANDTIME");
    let finish = referenced(plan, 12);
    assert_eq!(&*finish.type_name, "IFCLOCALTIME");
    assert_eq!(
        finish.attributes,
        vec![int(17), int(30), Value::Null, Value::Null, Value::Null]
    );
    assert_eq!(
        record(schedule).attributes[12],
        Value::Null,
        "no FinishTime"
    );
    // IfcRelSequence.TimeLag, slot 6, an IfcTimeMeasure in seconds.
    assert_eq!(record(sequence).attributes[6], Value::Real(86_400.0));
    let flush = written
        .iter()
        .map(|id| record(*id))
        .find(|entity| entity.attributes.get(2) == Some(&Value::Text("Flush".into())))
        .expect("USERDEFINED procedure");
    // IfcProcedure: ProcedureType 6, UserDefinedProcedureType 7.
    assert_eq!(flush.attributes[6], Value::Enum("USERDEFINED".into()));
    assert_eq!(flush.attributes[7], Value::Text("Flushing".into()));
}

/// The oracle is trusted because it fails when it should: an IFC2X3 task
/// with `$` for its required `OwnerHistory`, as the plain writer stages, is
/// an error finding.
#[test]
fn the_validator_catches_an_unowned_ifc2x3_task() {
    let mut model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let text = |s: &str| ifc::Value::Text(s.into());
    let null = ifc::Value::Null;
    let task = tx.create(ifc::Entity::new(
        "IFCTASK",
        vec![
            text(G[0]),
            null.clone(),
            text("Slab"),
            null.clone(),
            null.clone(),
            text("T-1"),
            null.clone(),
            null.clone(),
            ifc::Value::Bool(false),
            null,
        ],
    ));
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[task]).is_empty(),
        "IFC2X3 requires IfcRoot.OwnerHistory"
    );
}

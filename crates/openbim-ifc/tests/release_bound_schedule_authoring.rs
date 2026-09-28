//! Schedule records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#202).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-schedule`, written to STEP, read back with `ifc-step`, and checked
//! by `ifc-validate` against the declared release's table. No record this
//! test wrote may carry an error finding. IFC2X3 holds tasks, procedures,
//! nesting and control assignments; its work controls, sequences, events and
//! calendars cannot be authored through these signatures and are refused
//! (see `ifc-schedule`'s `release_refusals.rs`).

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
    EventDraft, ProcedureDraft, TaskDraft, WorkControlDraft, WorkControlKind,
};
use ifc::schema::{for_version, SchemaVersion};
use ifc::{Codec, Model, StepCodec};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);
/// The IFC2X3 work schedule [`base`] parses; IFC4 and IFC4X3 author theirs.
const IFC2X3_SCHEDULE: EntityId = EntityId(21);

const G: [&str; 9] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
    "0YvctVUKr0kugbFTf53O0C",
    "0YvctVUKr0kugbFTf53O0D",
    "0YvctVUKr0kugbFTf53O0E",
    "0YvctVUKr0kugbFTf53O0F",
    "0YvctVUKr0kugbFTf53O0G",
];

/// Actors and an owner history (`#5`) in `schema`; in IFC2X3 also a work
/// schedule (`#21`).
fn base(schema: &str, version: SchemaVersion) -> Model {
    let schedule = if version == SchemaVersion::Ifc2x3 {
        "#20=IFCCALENDARDATE(1,1,2026);\n\
         #21=IFCWORKSCHEDULE('2kTvXnbbzCWw8lcMd1dR4o',#5,'Programme',$,$,'WS-1',#20,$,$,$,$,#20,$,$,$);\n"
    } else {
        ""
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         {schedule}\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn task(global_id: &'static str, name: &'static str, ifc4: bool) -> TaskDraft<'static> {
    TaskDraft {
        global_id,
        name: Some(name),
        description: Some("Pour"),
        identification: Some("T-1"),
        long_description: ifc4.then_some("Long"),
        status: Some("PLANNED"),
        work_method: Some("Pump"),
        is_milestone: false,
        priority: Some(10),
        task_time: None,
        predefined_type: ifc4.then_some("CONSTRUCTION"),
    }
}

/// Every `IfcRoot` writer `version` can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let mut tx = Transaction::new(model);
    let t1 = create_task_with_owner_history(&mut tx, model, task(G[0], "Slab", ifc4), OWNER)
        .expect("task");
    let t2 = create_task_with_owner_history(&mut tx, model, task(G[1], "Walls", ifc4), OWNER)
        .expect("task");
    let procedure = ProcedureDraft {
        global_id: G[2],
        name: Some("Calibrate"),
        identification: Some("P-1"),
        predefined_type: Some("CALIBRATION"),
        ..ProcedureDraft::default()
    };
    let procedure =
        create_procedure_with_owner_history(&mut tx, model, procedure, OWNER).expect("procedure");
    let nest = nest_tasks_with_owner_history(&mut tx, model, G[3], t1, &[t2], OWNER).expect("nest");
    let mut written = vec![t1, t2, procedure, nest];
    let schedule = if ifc4 {
        let draft = WorkControlDraft {
            global_id: G[5],
            name: Some("Programme"),
            identification: Some("WS-1"),
            creation_date: "2026-09-28T00:00:00",
            start_time: "2026-10-01T08:00:00",
            predefined_type: Some("PLANNED"),
            ..WorkControlDraft::default()
        };
        let schedule = create_work_control_with_owner_history(
            &mut tx,
            model,
            WorkControlKind::Schedule,
            draft,
            OWNER,
        )
        .expect("schedule");
        let sequence = create_sequence_with_owner_history(
            &mut tx,
            model,
            G[6],
            t1,
            t2,
            Some("FINISH_START"),
            None,
            OWNER,
        )
        .expect("sequence");
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
        let event = EventDraft {
            global_id: G[8],
            name: Some("Handover"),
            predefined_type: Some("ENDEVENT"),
            trigger_type: Some("EVENTTIME"),
            ..EventDraft::default()
        };
        let event = create_event_with_owner_history(&mut tx, model, event, OWNER).expect("event");
        written.extend([schedule, sequence, calendar, event]);
        schedule
    } else {
        IFC2X3_SCHEDULE
    };
    let assigned =
        assign_tasks_to_control_with_owner_history(&mut tx, model, G[4], schedule, &[t1], OWNER)
            .expect("assignment");
    written.push(assigned);
    tx.commit(model).expect("commit");
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
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    }
}

/// The oracle is trusted because it fails when it should: an IFC2X3 task
/// with `$` for its required `OwnerHistory`, as the plain writer stages, is
/// an error finding.
#[test]
fn the_validator_catches_an_unowned_ifc2x3_task() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
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

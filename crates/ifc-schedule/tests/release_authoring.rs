//! Schedule authoring bound to the declared release (#202).
//!
//! From the EXPRESS sources: IFC2X3 TC1 requires `IfcRoot.OwnerHistory`,
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 make it `OPTIONAL`. IFC2X3 `IfcTask` has
//! ten attributes (`TaskId` required, no `LongDescription`, `TaskTime` or
//! `PredefinedType`) where IFC4 has thirteen; IFC2X3 `IfcProcedure` names
//! its identifier `ProcedureID` and its required type `ProcedureType`.

mod common;

use common::{base, staged, IFC2X3_SCHEDULE, OWNER, RELEASES};
use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_schedule::{
    assign_tasks_to_control, assign_tasks_to_control_with_owner_history, create_event,
    create_event_with_owner_history, create_procedure, create_procedure_with_owner_history,
    create_sequence, create_sequence_with_owner_history, create_task,
    create_task_with_owner_history, create_work_calendar, create_work_calendar_with_owner_history,
    create_work_control, create_work_control_with_owner_history, create_work_time, events,
    nest_tasks, nest_tasks_with_owner_history, subtasks_of, successors_of, tasks,
    tasks_of_schedule, work_calendars, work_schedules, EventDraft, ProcedureDraft, SchemaVersion,
    TaskDraft, TimeLag, WorkControlDraft, WorkControlKind,
};
use ifc_schema::for_version;
use ifc_step::StepCodec;

const G: [&str; 10] = [
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
];

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

fn schedule_draft() -> WorkControlDraft<'static> {
    WorkControlDraft::new(G[5], "2026-09-28T00:00:00", "2026-10-01T08:00:00")
        .name("Programme")
        .identification("WS-1")
        .predefined_type("PLANNED")
}

fn event_draft() -> EventDraft<'static> {
    EventDraft::new(G[7])
        .name("Handover")
        .predefined_type("ENDEVENT")
        .trigger_type("EVENTTIME")
}

fn procedure_draft() -> ProcedureDraft<'static> {
    ProcedureDraft::new(G[2])
        .name("Calibrate")
        .identification("P-1")
        .predefined_type("CALIBRATION")
}

/// Author what `version` can hold through the variants, commit, and return
/// every written record with its entity name.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<(&'static str, EntityId)> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let mut tx = Transaction::new(model);
    let t1 = create_task_with_owner_history(&mut tx, model, task(G[0], "Slab", ifc4), OWNER)
        .expect("task");
    let t2 = create_task_with_owner_history(&mut tx, model, task(G[1], "Walls", ifc4), OWNER)
        .expect("task");
    let procedure = create_procedure_with_owner_history(&mut tx, model, procedure_draft(), OWNER)
        .expect("procedure");
    let nest = nest_tasks_with_owner_history(&mut tx, model, G[3], t1, &[t2], OWNER).expect("nest");
    let mut written = vec![
        ("IFCTASK", t1),
        ("IFCTASK", t2),
        ("IFCPROCEDURE", procedure),
        ("IFCRELNESTS", nest),
    ];
    let schedule = if ifc4 {
        let schedule = create_work_control_with_owner_history(
            &mut tx,
            model,
            WorkControlKind::Schedule,
            schedule_draft(),
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
            G[8],
            Some("Day shift"),
            &[period],
            &[],
            Some("FIRSTSHIFT"),
            OWNER,
        )
        .expect("calendar");
        let event =
            create_event_with_owner_history(&mut tx, model, event_draft(), OWNER).expect("event");
        written.extend([
            ("IFCWORKSCHEDULE", schedule),
            ("IFCRELSEQUENCE", sequence),
            ("IFCWORKCALENDAR", calendar),
            ("IFCEVENT", event),
        ]);
        schedule
    } else {
        IFC2X3_SCHEDULE
    };
    let assigned =
        assign_tasks_to_control_with_owner_history(&mut tx, model, G[4], schedule, &[t1], OWNER)
            .expect("assignment");
    written.push(("IFCRELASSIGNSTOCONTROL", assigned));
    tx.commit(model).expect("commit");
    written
}

/// Each release round-trips: written, re-read with `ifc-step`, laid out by
/// name from its own table, and read back through the crate's views.
#[test]
fn schedule_records_round_trip_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let table = for_version(version).unwrap();
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for (entity, id) in &written {
            let record = back.get(*id).expect("read back");
            assert_eq!(record.type_name.as_ref(), *entity);
            let names = table.attribute_names(entity);
            assert_eq!(record.attributes.len(), names.len(), "{schema} {entity}");
            assert_eq!(record.attributes[1], Value::Ref(OWNER), "{schema} {entity}");
        }
        let at = |id: EntityId, name: &str| {
            let record = back.get(id).unwrap();
            let names = table.attribute_names(&record.type_name);
            record.attributes[names.iter().position(|n| *n == name).unwrap()].clone()
        };
        let (t1, t2, procedure) = (written[0].1, written[1].1, written[2].1);
        let (task_id, procedure_id, procedure_type) = if version == SchemaVersion::Ifc2x3 {
            ("TaskId", "ProcedureID", "ProcedureType")
        } else {
            ("Identification", "Identification", "PredefinedType")
        };
        assert_eq!(at(t1, task_id), Value::Text("T-1".into()), "{schema}");
        assert_eq!(at(procedure, procedure_id), Value::Text("P-1".into()));
        assert_eq!(
            at(procedure, procedure_type),
            Value::Enum("CALIBRATION".into())
        );

        // The views. Relationships and names read the same slots in every
        // release; IFC2X3 task attributes past TaskId are not asserted
        // through the IFC4-positioned `Task` accessors.
        let names: Vec<_> = tasks(&back)
            .expect("bound")
            .iter()
            .filter_map(|t| t.name())
            .collect();
        assert_eq!(names, ["Slab", "Walls"], "{schema}");
        assert_eq!(subtasks_of(&back, t1).expect("bound"), [t2], "{schema}");
        let schedule = written
            .iter()
            .find(|(e, _)| *e == "IFCWORKSCHEDULE")
            .map_or(common::IFC2X3_SCHEDULE, |(_, id)| *id);
        assert_eq!(
            tasks_of_schedule(&back, schedule).expect("bound"),
            [t1],
            "{schema}"
        );
        if version != SchemaVersion::Ifc2x3 {
            let task = tasks(&back)
                .expect("bound")
                .into_iter()
                .find(|t| t.id() == t1)
                .unwrap();
            assert_eq!(task.identification(), Some("T-1"));
            assert_eq!(task.predefined_type(), Some("CONSTRUCTION"));
            assert_eq!(successors_of(&back, t1).expect("bound"), [t2], "{schema}");
            assert_eq!(
                work_schedules(&back).expect("bound")[0]
                    .start_time()
                    .and_then(|d| d.text()),
                Some("2026-10-01T08:00:00")
            );
            assert_eq!(work_calendars(&back).len(), 1, "{schema}");
            assert_eq!(events(&back)[0].trigger_type(), Some("EVENTTIME"));
        }
    }
}

/// IFC4 and IFC4X3 output is unchanged: each plain writer stages the
/// pre-#202 positional record, and its variant the same record with the
/// owner history in slot 1.
#[test]
#[allow(clippy::too_many_lines)]
fn ifc4_and_ifc4x3_records_are_unchanged() {
    let t = |s: &str| Value::Text(s.into());
    let e = |s: &str| Value::Enum(s.into());
    let r = Value::Ref;
    let n = || Value::Null;
    for (schema, version) in &RELEASES[1..] {
        let model = base(schema, *version);
        let mut tx = Transaction::new(&model);
        let mut pairs = Vec::new();
        let p = create_task(&mut tx, task(G[0], "Slab", true)).unwrap();
        let v = create_task_with_owner_history(&mut tx, &model, task(G[0], "Slab", true), OWNER)
            .unwrap();
        let expected = vec![
            t(G[0]),
            n(),
            t("Slab"),
            t("Pour"),
            n(),
            t("T-1"),
            t("Long"),
            t("PLANNED"),
            t("Pump"),
            Value::Bool(false),
            Value::Integer(10),
            n(),
            e("CONSTRUCTION"),
        ];
        pairs.push((p, v, expected));
        let (a, b) = (EntityId(10), EntityId(11));
        let p = create_sequence(
            &mut tx,
            G[6],
            a,
            b,
            Some("FINISH_START"),
            Some(EntityId(12)),
        )
        .unwrap();
        let v = create_sequence_with_owner_history(
            &mut tx,
            &model,
            G[6],
            a,
            b,
            Some("FINISH_START"),
            Some(TimeLag::LagTime(EntityId(12))),
            OWNER,
        )
        .unwrap();
        let expected = vec![
            t(G[6]),
            n(),
            n(),
            n(),
            r(a),
            r(b),
            r(EntityId(12)),
            e("FINISH_START"),
            n(),
        ];
        pairs.push((p, v, expected));
        let kind = WorkControlKind::Plan;
        let p = create_work_control(&mut tx, kind, schedule_draft()).unwrap();
        let v =
            create_work_control_with_owner_history(&mut tx, &model, kind, schedule_draft(), OWNER)
                .unwrap();
        let expected = vec![
            t(G[5]),
            n(),
            t("Programme"),
            n(),
            n(),
            t("WS-1"),
            t("2026-09-28T00:00:00"),
            n(),
            n(),
            n(),
            n(),
            t("2026-10-01T08:00:00"),
            n(),
            e("PLANNED"),
        ];
        pairs.push((p, v, expected));
        let p = assign_tasks_to_control(&mut tx, G[4], a, &[b]).unwrap();
        let v = assign_tasks_to_control_with_owner_history(&mut tx, &model, G[4], a, &[b], OWNER)
            .unwrap();
        let expected = vec![t(G[4]), n(), n(), n(), Value::List(vec![r(b)]), n(), r(a)];
        pairs.push((p, v, expected));
        let p = nest_tasks(&mut tx, G[3], a, &[b]).unwrap();
        let v = nest_tasks_with_owner_history(&mut tx, &model, G[3], a, &[b], OWNER).unwrap();
        let expected = vec![t(G[3]), n(), n(), n(), r(a), Value::List(vec![r(b)])];
        pairs.push((p, v, expected));
        let p = create_work_calendar(&mut tx, G[8], Some("Day"), &[a], &[], Some("FIRSTSHIFT"))
            .unwrap();
        let v = create_work_calendar_with_owner_history(
            &mut tx,
            &model,
            G[8],
            Some("Day"),
            &[a],
            &[],
            Some("FIRSTSHIFT"),
            OWNER,
        )
        .unwrap();
        let expected = vec![
            t(G[8]),
            n(),
            t("Day"),
            n(),
            n(),
            n(),
            Value::List(vec![r(a)]),
            n(),
            e("FIRSTSHIFT"),
        ];
        pairs.push((p, v, expected));
        let p = create_event(&mut tx, event_draft()).unwrap();
        let v = create_event_with_owner_history(&mut tx, &model, event_draft(), OWNER).unwrap();
        let expected = vec![
            t(G[7]),
            n(),
            t("Handover"),
            n(),
            n(),
            n(),
            n(),
            e("ENDEVENT"),
            e("EVENTTIME"),
            n(),
            n(),
        ];
        pairs.push((p, v, expected));
        let p = create_procedure(&mut tx, procedure_draft()).unwrap();
        let v =
            create_procedure_with_owner_history(&mut tx, &model, procedure_draft(), OWNER).unwrap();
        let expected = vec![
            t(G[2]),
            n(),
            t("Calibrate"),
            n(),
            n(),
            t("P-1"),
            n(),
            e("CALIBRATION"),
        ];
        pairs.push((p, v, expected));

        for (plain, variant, mut expected) in pairs {
            let record = staged(&tx, plain);
            assert_eq!(record.attributes, expected, "{schema} {}", record.type_name);
            expected[1] = Value::Ref(OWNER);
            assert_eq!(staged(&tx, variant).attributes, expected, "{schema}");
        }
    }
}

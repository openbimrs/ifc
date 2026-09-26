//! A whole programme authored, then read back by this crate's readers.
//!
//! The existing authoring tests cover tasks in isolation. A programme is
//! the thing a consumer actually wants: a schedule, its tasks, the
//! nesting between them and a calendar saying when work may happen. If a
//! slot layout is wrong anywhere in that graph, the queries return the
//! wrong set rather than failing loudly, so each helper is checked
//! through the reader that consumes it.

use ifc_model::codec::Codec;
use ifc_model::{Model, Transaction};
use ifc_schedule::schedule::{work_schedules, WorkControlKind};
use ifc_schedule::{
    assign_tasks_to_control, create_task, create_work_calendar, create_work_control,
    create_work_time, nest_tasks, subtasks_of, tasks_of_schedule, work_calendars, TaskDraft,
    WorkControlDraft,
};

fn task(tx: &mut Transaction, guid: &str, name: &str) -> ifc_model::EntityId {
    create_task(
        tx,
        TaskDraft {
            global_id: guid,
            name: Some(name),
            is_milestone: false,
            ..TaskDraft::default()
        },
    )
    .expect("authored task")
}

/// A schedule, its tasks and their nesting read back as authored.
#[test]
fn an_authored_programme_reads_back_through_the_queries() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let schedule = create_work_control(
        &mut tx,
        WorkControlKind::Schedule,
        WorkControlDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Programme"),
            creation_date: "2026-01-05T08:00:00",
            start_time: "2026-01-05T08:00:00",
            finish_time: Some("2026-06-30T17:00:00"),
            ..WorkControlDraft::default()
        },
    )
    .expect("authored schedule");

    let parent = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "Substructure");
    let first = task(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", "Excavate");
    let second = task(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", "Pour");

    assign_tasks_to_control(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", schedule, &[parent])
        .expect("assigned");
    nest_tasks(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", parent, &[first, second]).expect("nested");
    tx.commit(&mut model).expect("commit");

    let found = work_schedules(&model);
    assert_eq!(found.len(), 1, "one schedule");
    assert_eq!(found[0].name(), Some("Programme"));
    assert_eq!(found[0].start_time(), Some("2026-01-05T08:00:00"));

    assert_eq!(tasks_of_schedule(&model, schedule), vec![parent]);
    assert_eq!(subtasks_of(&model, parent), vec![first, second]);
}

/// A calendar with working and exception periods reads back in both roles.
#[test]
fn an_authored_calendar_separates_working_from_exception_time() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let shift = create_work_time(
        &mut tx,
        Some("Weekday shift"),
        None,
        Some("2026-01-05T08:00:00"),
        Some("2026-12-18T17:00:00"),
    )
    .expect("authored working time");
    let holiday = create_work_time(
        &mut tx,
        Some("Shutdown"),
        None,
        Some("2026-08-03T00:00:00"),
        Some("2026-08-14T23:59:59"),
    )
    .expect("authored exception time");

    let calendar = create_work_calendar(
        &mut tx,
        "06BcDeFgHiJkLmNoPqRsTu",
        Some("Site calendar"),
        &[shift],
        &[holiday],
        Some("FIRSTSHIFT"),
    )
    .expect("authored calendar");
    tx.commit(&mut model).expect("commit");

    let found = work_calendars(&model);
    assert_eq!(found.len(), 1, "one calendar");
    assert_eq!(found[0].id(), calendar);

    // The two roles must not collapse into one list: a shutdown read as
    // working time inverts the meaning of the calendar.
    let working: Vec<_> = found[0]
        .working_times(&model)
        .into_iter()
        .map(|t| t.id)
        .collect();
    let exceptions: Vec<_> = found[0]
        .exception_times(&model)
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(working, vec![shift]);
    assert_eq!(exceptions, vec![holiday]);
}

/// Values that parse and validate but mean nothing are refused.
///
/// Each of these writes a file that loads: the damage is semantic, so the
/// author is the only place it can be caught.
#[test]
fn values_that_would_parse_but_mean_nothing_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let anchor = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "Anchor");

    // A schedule that starts at no stated time schedules nothing.
    assert!(create_work_control(
        &mut tx,
        WorkControlKind::Schedule,
        WorkControlDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            creation_date: "2026-01-05T08:00:00",
            start_time: "   ",
            ..WorkControlDraft::default()
        },
    )
    .is_err());

    // An assignment relating no objects assigns nothing.
    assert!(assign_tasks_to_control(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", anchor, &[]).is_err());

    // A task nested under itself is a cycle the timeline cannot walk.
    assert!(nest_tasks(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", anchor, &[anchor]).is_err());

    // A calendar constraining no period reads as a real calendar.
    assert!(create_work_calendar(&mut tx, "06BcDeFgHiJkLmNoPqRsTu", None, &[], &[], None).is_err());

    // A GUID that is not a GUID makes the record unaddressable.
    assert!(create_work_calendar(&mut tx, "not-a-guid", None, &[anchor], &[], None).is_err());
}

/// The programme survives serialisation to STEP text and back.
///
/// The tests above stay in memory. A consumer writes a file, so the
/// writer and parser sit between authoring and reading; a slot that
/// survives one but not the other is invisible until then.
#[test]
fn an_authored_programme_survives_step_text() {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };
    let mut tx = Transaction::new(&model);
    let schedule = create_work_control(
        &mut tx,
        WorkControlKind::Schedule,
        WorkControlDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Programme"),
            creation_date: "2026-01-05T08:00:00",
            start_time: "2026-01-05T08:00:00",
            ..WorkControlDraft::default()
        },
    )
    .expect("authored schedule");
    let parent = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "Substructure");
    let child = task(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", "Excavate");
    assign_tasks_to_control(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", schedule, &[parent])
        .expect("assigned");
    nest_tasks(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", parent, &[child]).expect("nested");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    ifc_step::StepCodec
        .write(&model, &mut bytes)
        .expect("written");
    let reparsed = ifc_step::StepCodec.read_bytes(&bytes).expect("reparsed");

    let found = work_schedules(&reparsed);
    assert_eq!(found.len(), 1, "one schedule after the round trip");
    assert_eq!(found[0].name(), Some("Programme"));
    // Entity ids are assigned by the parser, so compare the shape of the
    // graph rather than the ids the authoring transaction happened to use.
    let tasks = tasks_of_schedule(&reparsed, found[0].id());
    assert_eq!(tasks.len(), 1, "the assignment survived");
    assert_eq!(
        subtasks_of(&reparsed, tasks[0]).len(),
        1,
        "nesting survived"
    );
}

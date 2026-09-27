//! Authoring recurring task times and time periods.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schedule::{
    create_recurrence_pattern, create_task_time, create_task_time_recurring, create_time_period,
    RecurrenceDraft, TaskTimeDraft,
};

fn draft() -> TaskTimeDraft<'static> {
    TaskTimeDraft {
        name: Some("Weekly inspection"),
        duration_type: Some("WORKTIME"),
        schedule_duration: Some("PT2H"),
        schedule_start: None,
        schedule_finish: None,
        actual_start: None,
        actual_finish: None,
        is_critical: None,
        completion: None,
    }
}

/// The recurring form adds Recurrence at slot 20.
#[test]
fn a_recurring_task_time_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let pattern = create_recurrence_pattern(
        &mut tx,
        &RecurrenceDraft {
            recurrence_type: "WEEKLY",
            ..RecurrenceDraft::default()
        },
    )
    .expect("pattern");

    let plain = create_task_time(&mut tx, draft()).expect("task time");
    let recurring =
        create_task_time_recurring(&mut tx, &model, draft(), pattern).expect("recurring task time");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(plain).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCTASKTIME");

    let staged = model.get(recurring).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCTASKTIMERECURRING");
    assert_eq!(staged.attributes.len(), 21);
    assert_eq!(staged.attributes[20], Value::Ref(pattern));
    // The inherited slots survive the extension.
    assert_eq!(
        staged.attributes[0],
        Value::Text("Weekly inspection".into())
    );
}

/// A recurrence that is not a pattern is refused.
#[test]
fn a_non_pattern_recurrence_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let stray = tx.create(Entity::new("IFCTASKTIME", vec![Value::Null; 20]));

    assert!(
        create_task_time_recurring(&mut tx, &model, draft(), stray).is_err(),
        "an IfcTaskTime was accepted as a recurrence pattern",
    );
}

/// A time period stages both required times.
#[test]
fn a_time_period_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_time_period(&mut tx, "08:00:00", "17:00:00").expect("time period");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCTIMEPERIOD");
    assert_eq!(staged.attributes.len(), 2);
    assert_eq!(staged.attributes[0], Value::Text("08:00:00".into()));
    assert_eq!(staged.attributes[1], Value::Text("17:00:00".into()));
}

/// A blank time is refused: both slots are required.
#[test]
fn a_blank_time_period_bound_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(create_time_period(&mut tx, "", "17:00:00").is_err());
    assert!(create_time_period(&mut tx, "08:00:00", "   ").is_err());
}

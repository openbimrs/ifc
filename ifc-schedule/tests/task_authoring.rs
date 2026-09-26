//! Task, task-time and sequence authoring.
//!
//! IfcTask carries thirteen slots but declares only six; the rest are
//! inherited from IfcRoot, IfcObject and IfcProcess. These tests commit an
//! authored programme and read it back through the crate's own readers and
//! execution-order traversal, so a slot-counting mistake fails here rather
//! than in a downstream file.

use ifc_model::{Model, Transaction};
use ifc_schedule::{
    create_sequence, create_task, create_task_time, execution_order, tasks, TaskDraft,
    TaskTimeDraft,
};

fn task(tx: &mut Transaction, guid: &str, name: &'static str) -> ifc_model::EntityId {
    create_task(
        tx,
        TaskDraft {
            global_id: guid,
            name: Some(name),
            is_milestone: false,
            ..TaskDraft::default()
        },
    )
    .expect("task")
}

#[test]
fn an_authored_task_reads_back_through_the_task_reader() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let time = create_task_time(
        &mut tx,
        TaskTimeDraft {
            name: Some("Pour"),
            duration_type: Some("WORKTIME"),
            schedule_duration: Some("P5D"),
            schedule_start: Some("2026-01-05T08:00:00"),
            completion: Some(40.0),
            ..TaskTimeDraft::default()
        },
    )
    .expect("task time");
    let id = create_task(
        &mut tx,
        TaskDraft {
            global_id: "0aBcDeFgHiJkLmNoPqRsTu",
            name: Some("Slab"),
            description: Some("Ground floor"),
            identification: Some("T-01"),
            status: Some("NOTSTARTED"),
            is_milestone: false,
            priority: Some(20),
            task_time: Some(time),
            ..TaskDraft::default()
        },
    )
    .expect("task");
    tx.commit(&mut model).expect("commit");
    let found = tasks(&model)
        .into_iter()
        .find(|t| t.id() == id)
        .expect("readable");
    assert_eq!(found.name(), Some("Slab"), "Name is slot 2, not slot 0");
    assert_eq!(
        found.identification(),
        Some("T-01"),
        "Identification is slot 5"
    );
    assert_eq!(found.status(), Some("NOTSTARTED"), "Status is slot 7");
    assert_eq!(found.is_milestone(), Some(false));
    assert_eq!(found.priority(), Some(20));
    assert_eq!(found.task_time_ref(), Some(time), "TaskTime is slot 11");
    let (t, anomalies) = found.time(&model);
    let t = t.expect("task time readable");
    assert!(
        anomalies.is_empty(),
        "authored time must be self-consistent: {anomalies:?}"
    );
    assert_eq!(t.schedule_duration(), Some("P5D"));
}

#[test]
fn an_authored_sequence_drives_the_execution_order_traversal() {
    // The strongest evidence the links are right: the crate's own
    // topological traversal, which knows nothing about how they were
    // written, must order the tasks A -> B -> C.
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = task(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", "A");
    let b = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "B");
    let c = task(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", "C");
    create_sequence(
        &mut tx,
        "3aBcDeFgHiJkLmNoPqRsTu",
        a,
        b,
        Some("FINISH_START"),
        None,
    )
    .expect("a before b");
    create_sequence(
        &mut tx,
        "04BcDeFgHiJkLmNoPqRsTu",
        b,
        c,
        Some("FINISH_START"),
        None,
    )
    .expect("b before c");
    tx.commit(&mut model).expect("commit");
    let order = execution_order(&model).expect("an authored chain must not look cyclic");
    let positions: Vec<usize> = [a, b, c]
        .iter()
        .map(|id| order.iter().position(|t| t == id).expect("task ordered"))
        .collect();
    assert!(
        positions[0] < positions[1] && positions[1] < positions[2],
        "authored predecessors must sort before successors, got {positions:?}"
    );
}

#[test]
fn a_task_cannot_precede_itself() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let a = task(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", "A");
    assert!(
        create_sequence(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", a, a, None, None).is_err(),
        "a self-loop is an unsatisfiable constraint"
    );
}

#[test]
fn out_of_range_authored_values_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        create_task(
            &mut tx,
            TaskDraft {
                global_id: "0aBcDeFgHiJkLmNoPqRsTu",
                priority: Some(150),
                ..TaskDraft::default()
            },
        )
        .is_err(),
        "Priority is a 0..=100 scale"
    );
    assert!(
        create_task(
            &mut tx,
            TaskDraft {
                global_id: "not-a-guid",
                ..TaskDraft::default()
            }
        )
        .is_err(),
        "GlobalId must be a compressed GUID"
    );
    assert!(
        create_task_time(
            &mut tx,
            TaskTimeDraft {
                completion: Some(120.0),
                ..TaskTimeDraft::default()
            },
        )
        .is_err(),
        "Completion is a percentage"
    );
    assert!(
        create_task_time(
            &mut tx,
            TaskTimeDraft {
                duration_type: Some("SOMEDAY"),
                ..TaskTimeDraft::default()
            },
        )
        .is_err(),
        "DurationType is a closed enumeration"
    );
}

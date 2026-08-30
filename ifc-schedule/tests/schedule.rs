//! `ifc-schedule` over a real fixture: tasks, sequences, calendars, ordering.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_schedule::{
    end_tasks, events, execution_order, find_cycle, predecessors_of, sequences, start_tasks,
    subtasks_of, successors_of, tasks, tasks_of_schedule, work_calendars, work_plans,
    work_schedules, DurationType, RecurrenceType, SequenceType, Task, TaskTimeAnomaly,
    WorkControlKind, WorkTimeRole,
};

fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-cost-schedule/synthetic_cost_schedule.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn task_named<'m>(model: &'m Model, name: &str) -> Task<'m> {
    tasks(model)
        .into_iter()
        .find(|t| t.name() == Some(name))
        .expect("task in fixture")
}

// ---- SCHED-ROOT ----------------------------------------------------------

/// Plans and schedules share every slot through `IfcWorkControl`.
///
/// `PredefinedType` is slot 13, after six inherited slots and seven from
/// `IfcWorkControl`. A reader placing it right after `Identification` -- where
/// most `IfcControl` subtypes put it -- lands on `CreationDate` and reports a
/// timestamp as a type token.
#[test]
fn work_controls_read_their_inherited_slots() {
    let model = fixture();

    let plans = work_plans(&model);
    assert_eq!(plans.len(), 1);
    let plan = plans[0];
    assert_eq!(plan.kind(), WorkControlKind::Plan);
    assert_eq!(plan.name(), Some("Programme"));
    assert_eq!(plan.identification(), Some("WP-1"), "slot 5");
    assert_eq!(plan.creation_date(), Some("2026-01-10T08:00:00"), "slot 6");
    assert_eq!(plan.start_time(), Some("2026-03-02T08:00:00"), "slot 11");
    assert_eq!(plan.finish_time(), Some("2026-06-30T17:00:00"), "slot 12");
    assert_eq!(plan.predefined_type(), Some("PLANNED"), "slot 13, not 6");

    let schedules = work_schedules(&model);
    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0].kind(), WorkControlKind::Schedule);
    assert_eq!(schedules[0].identification(), Some("WS-1"));
}

/// Tasks reach their schedule through `IfcRelAssignsToControl`.
#[test]
fn a_schedule_holds_its_tasks() {
    let model = fixture();
    let schedule = work_schedules(&model)[0];

    let members = tasks_of_schedule(&model, schedule.id());
    assert_eq!(members.len(), 5, "every task is assigned");
    for id in members {
        let entity = model.get(id).expect("member resolves");
        assert_eq!(entity.type_name.as_ref(), "IFCTASK");
    }
}

// ---- SCHED-TASK ----------------------------------------------------------

/// `IfcTask` slots, including the required `IsMilestone` at 9.
#[test]
fn task_slots_match_the_schema() {
    let model = fixture();
    let excavate = task_named(&model, "Excavate");

    assert_eq!(excavate.identification(), Some("T-1"), "slot 5");
    assert_eq!(excavate.is_milestone(), Some(false), "slot 9, required");
    assert_eq!(excavate.predefined_type(), Some("CONSTRUCTION"), "slot 12");
    assert!(excavate.task_time_ref().is_some(), "slot 11 is a reference");
}

/// Task time reads its planned and actual fields from the right slots.
#[test]
fn task_time_reads_schedule_and_float() {
    let model = fixture();
    let clad = task_named(&model, "Install cladding");
    let (time, anomalies) = clad.time(&model);
    let time = time.expect("cladding states a time");

    assert!(anomalies.is_empty(), "no contradiction here");
    assert_eq!(time.duration_type(), Some(DurationType::WorkTime));
    assert_eq!(time.schedule_duration(), Some("P10D"), "slot 4");
    assert_eq!(time.schedule_start(), Some("2026-03-16T08:00:00"), "slot 5");
    assert_eq!(time.free_float(), Some("P2D"), "slot 11");
    assert_eq!(time.total_float(), Some("P4D"), "slot 12");
    assert_eq!(time.is_critical(), Some(false), "slot 13");
    assert_eq!(time.completion(), Some(25.0), "slot 19");
}

/// A milestone that states a duration contradicts `IfcTaskTime` WR1.
///
/// IfcOpenShell's validator does not catch this -- the fixture passes with no
/// issues -- so the crate reports it or nobody does.
#[test]
fn a_milestone_with_a_duration_is_an_anomaly() {
    let model = fixture();
    let handover = task_named(&model, "Practical completion");
    assert_eq!(handover.is_milestone(), Some(true));

    let (time, anomalies) = handover.time(&model);
    assert!(time.is_some(), "the time is still readable");
    assert_eq!(anomalies.len(), 1, "exactly one contradiction");
    match &anomalies[0] {
        TaskTimeAnomaly::MilestoneWithDuration { duration, .. } => {
            assert_eq!(duration, "P1D", "the duration is reported, not dropped");
        }
        other => panic!("expected WR1 anomaly, got {other:?}"),
    }
}

/// A `TaskTime` slot pointing at the wrong type is reported, not read.
#[test]
fn a_task_time_pointing_elsewhere_is_reported() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![Value::Text("w".into())]));
    let task = model.push(Entity::new(
        "IFCTASK",
        vec![
            Value::Text("t".into()),
            Value::Null,
            Value::Text("Bad".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Bool(false),
            Value::Null,
            Value::Ref(wall),
        ],
    ));

    let (time, anomalies) = Task::new(task, model.get(task).unwrap()).time(&model);
    assert!(time.is_none(), "a wall is not a task time");
    assert!(matches!(
        anomalies.as_slice(),
        [TaskTimeAnomaly::NotATaskTime { found, .. }] if found == "IFCWALL"
    ));
}

// ---- SCHED-SEQ -----------------------------------------------------------

/// Sequences are directed, typed, and carry their lag.
#[test]
fn sequences_state_direction_type_and_lag() {
    let model = fixture();
    let all = sequences(&model);
    assert_eq!(all.len(), 4);

    let pour = task_named(&model, "Pour foundations");
    let clad = task_named(&model, "Install cladding");
    let curing = all
        .iter()
        .find(|s| s.predecessor == pour.id() && s.successor == clad.id())
        .expect("pour precedes cladding");

    assert_eq!(curing.sequence_type, Some(SequenceType::FinishStart));
    let lag = curing.lag.as_ref().expect("curing states a lag");
    assert_eq!(lag.duration.as_deref(), Some("P2D"));
    assert_eq!(lag.ratio, None, "stated as a duration, not a ratio");
}

/// A `START_START` link is not finish-to-start, and the difference is stated.
///
/// A tool that assumes every sequence is finish-to-start schedules fit-out
/// after cladding completes rather than alongside it.
#[test]
fn sequence_type_distinguishes_start_start_from_finish_start() {
    let model = fixture();
    let clad = task_named(&model, "Install cladding");
    let fitout = task_named(&model, "Fit out");

    let link = sequences(&model)
        .into_iter()
        .find(|s| s.predecessor == clad.id() && s.successor == fitout.id())
        .expect("cladding precedes fit-out");
    assert_eq!(link.sequence_type, Some(SequenceType::StartStart));
}

/// Predecessor and successor queries are inverses.
#[test]
fn predecessors_and_successors_agree() {
    let model = fixture();
    let excavate = task_named(&model, "Excavate");
    let pour = task_named(&model, "Pour foundations");

    assert_eq!(successors_of(&model, excavate.id()), vec![pour.id()]);
    assert_eq!(predecessors_of(&model, pour.id()), vec![excavate.id()]);
    assert!(
        predecessors_of(&model, excavate.id()).is_empty(),
        "excavation starts the chain"
    );
}

/// A sequence cycle is reported with its path.
#[test]
fn a_sequence_cycle_is_reported() {
    let mut model = Model::new();
    let mut task = |name: &str| {
        model.push(Entity::new(
            "IFCTASK",
            vec![
                Value::Text(name.into()),
                Value::Null,
                Value::Text(name.into()),
            ],
        ))
    };
    let a = task("A");
    let b = task("B");
    for (pred, succ) in [(a, b), (b, a)] {
        model.push(Entity::new(
            "IFCRELSEQUENCE",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(pred),
                Value::Ref(succ),
            ],
        ));
    }

    let cycle = find_cycle(&model).expect("the graph loops");
    assert!(
        cycle.path.len() >= 2,
        "the path is reported: {:?}",
        cycle.path
    );
    assert!(execution_order(&model).is_err(), "no valid ordering exists");
}

// ---- SCHED-CAL -----------------------------------------------------------

/// Working times and exception times are the same type in opposite slots.
///
/// Collecting every `IfcWorkTime` uniformly turns the Easter shutdown into
/// four working days.
#[test]
fn working_and_exception_times_are_not_interchangeable() {
    let model = fixture();
    let calendars = work_calendars(&model);
    assert_eq!(calendars.len(), 1);
    let calendar = calendars[0];
    assert_eq!(calendar.identification(), Some("CAL-1"));
    assert_eq!(calendar.predefined_type(), Some("FIRSTSHIFT"), "slot 8");

    let working = calendar.working_times(&model);
    let exceptions = calendar.exception_times(&model);
    assert_eq!(working.len(), 1);
    assert_eq!(exceptions.len(), 1);
    assert_eq!(working[0].role, WorkTimeRole::Working);
    assert_eq!(exceptions[0].role, WorkTimeRole::Exception);
    assert_eq!(exceptions[0].name.as_deref(), Some("Easter"));
}

/// A recurrence pattern is reported as authored, including being unbounded.
#[test]
fn an_unbounded_recurrence_is_reported_not_expanded() {
    let model = fixture();
    let calendar = work_calendars(&model)[0];
    let working = calendar.working_times(&model);
    let recurrence = working[0].recurrence.as_ref().expect("weekdays recur");

    assert_eq!(recurrence.recurrence_type, Some(RecurrenceType::Weekly));
    assert_eq!(recurrence.weekdays, vec![1, 2, 3, 4, 5], "Monday to Friday");
    assert_eq!(recurrence.interval, Some(1));
    assert!(
        !recurrence.is_bounded(),
        "no Occurrences stated, so expansion needs a caller-supplied window"
    );
}

// ---- SCHED-EVENT ---------------------------------------------------------

/// Events state instants, with the schema's own `EventOccurenceTime` spelling.
#[test]
fn an_event_states_scheduled_and_actual_dates() {
    let model = fixture();
    let all = events(&model);
    assert_eq!(all.len(), 1);
    let event = all[0];

    assert_eq!(event.name(), Some("Foundation inspection"));
    assert_eq!(event.predefined_type(), Some("INTERMEDIATEEVENT"), "slot 7");
    assert_eq!(event.trigger_type(), Some("EVENTRULE"), "slot 8");

    let time = event.time(&model).expect("slot 10 resolves");
    assert_eq!(time.scheduled.as_deref(), Some("2026-03-12T10:00:00"));
    assert_eq!(time.actual.as_deref(), Some("2026-03-12T14:00:00"));
}

// ---- SCHED-QUERY ---------------------------------------------------------

/// Execution order respects every sequence link.
#[test]
fn execution_order_is_a_valid_topological_sort() {
    let model = fixture();
    let order = execution_order(&model).expect("the fixture is acyclic");
    assert_eq!(order.len(), 5, "every task is ordered");

    let position = |id: EntityId| order.iter().position(|o| *o == id).expect("ordered");
    for link in sequences(&model) {
        assert!(
            position(link.predecessor) < position(link.successor),
            "predecessor precedes successor"
        );
    }
}

/// The order is stable against the model's internal iteration order.
///
/// Re-running the same query proves nothing: it would repeat whatever order
/// the first run produced. What matters is that two models with the SAME
/// tasks and links, built in different insertion orders, agree on the parts
/// the sequence graph actually constrains -- and that ready tasks are emitted
/// by file position rather than by however the map happens to iterate.
#[test]
fn execution_order_is_deterministic() {
    // Four tasks, two independent chains: A->B and C->D. Nothing links the
    // chains, so a nondeterministic tie-break is free to interleave them.
    fn build(order: &[&str]) -> (Model, Vec<EntityId>) {
        let mut model = Model::new();
        let mut ids = Vec::new();
        for name in order {
            ids.push(model.push(Entity::new(
                "IFCTASK",
                vec![
                    Value::Text((*name).into()),
                    Value::Null,
                    Value::Text((*name).into()),
                ],
            )));
        }
        (model, ids)
    }

    let (mut model, ids) = build(&["A", "B", "C", "D"]);
    for (pred, succ) in [(ids[0], ids[1]), (ids[2], ids[3])] {
        model.push(Entity::new(
            "IFCRELSEQUENCE",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(pred),
                Value::Ref(succ),
            ],
        ));
    }

    let order = execution_order(&model).expect("acyclic");
    // File order decides between the two independent chains, so the result is
    // fully determined: A, C, B, D would also be a valid topological sort, but
    // only one of them is the stated one.
    assert_eq!(
        order,
        vec![ids[0], ids[1], ids[2], ids[3]],
        "ready tasks are emitted in file order, not map order"
    );

    // Same graph, tasks inserted in a different order: the emitted sequence
    // must follow the new file order, proving the tie-break reads position
    // rather than preserving a coincidence.
    let (mut reordered, rids) = build(&["C", "D", "A", "B"]);
    for (pred, succ) in [(rids[2], rids[3]), (rids[0], rids[1])] {
        reordered.push(Entity::new(
            "IFCRELSEQUENCE",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(pred),
                Value::Ref(succ),
            ],
        ));
    }
    let reordered_order = execution_order(&reordered).expect("acyclic");
    assert_eq!(
        reordered_order,
        vec![rids[0], rids[1], rids[2], rids[3]],
        "file order drives the result, so C's chain now comes first"
    );
}

/// Start and end tasks bound the schedule.
#[test]
fn start_and_end_tasks_are_the_graph_boundary() {
    let model = fixture();
    let excavate = task_named(&model, "Excavate");
    let handover = task_named(&model, "Practical completion");

    assert_eq!(start_tasks(&model), vec![excavate.id()]);
    assert_eq!(end_tasks(&model), vec![handover.id()]);
}

/// Tasks nest into a work breakdown, independently of sequencing.
#[test]
fn tasks_nest_independently_of_sequence() {
    let model = fixture();
    let excavate = task_named(&model, "Excavate");
    let pour = task_named(&model, "Pour foundations");

    assert_eq!(
        subtasks_of(&model, excavate.id()),
        vec![pour.id()],
        "nesting is a separate relationship from sequencing"
    );
}

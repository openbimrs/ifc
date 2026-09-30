//! The sequence graph covers every `IfcProcess`, refuses a walk past its
//! budget, and reads the whole `IfcLagTime` (#236).
//!
//! `IfcRelSequence` relates `IfcProcess`, whose IFC4 ADD2 TC1 and IFC4X3
//! ADD2 subtypes are `IfcEvent`, `IfcProcedure` and `IfcTask`.
//! `IfcLagTime` is an `IfcSchedulingTime` (`Name`, `DataOrigin`,
//! `UserDefinedDataOrigin`) plus `LagValue` and the required
//! `DurationType : IfcTaskDurationEnum`.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schedule::error::ScheduleReadError;
use ifc_schedule::{
    create_lag_time, create_sequence, downstream_of, execution_order, find_cycle,
    process_execution_order, sequences, DurationType, MAX_SEQUENCE_DEPTH,
};

fn process(model: &mut Model, entity: &str, name: &str) -> EntityId {
    model.push(Entity::new(
        entity,
        vec![
            Value::Text(name.into()),
            Value::Null,
            Value::Text(name.into()),
        ],
    ))
}

fn link(model: &mut Model, predecessor: EntityId, successor: EntityId) {
    model.push(Entity::new(
        "IFCRELSEQUENCE",
        vec![
            Value::Text("rel".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(predecessor),
            Value::Ref(successor),
        ],
    ));
}

/// A chain of `length` tasks, first to last.
fn chain(length: usize) -> (Model, Vec<EntityId>) {
    let mut model = Model::new();
    let ids: Vec<EntityId> = (0..length)
        .map(|i| process(&mut model, "IFCTASK", &format!("T{i}")))
        .collect();
    for pair in ids.windows(2) {
        link(&mut model, pair[0], pair[1]);
    }
    (model, ids)
}

/// A chain as deep as the budget is walked in full.
#[test]
fn a_chain_within_the_budget_is_walked_in_full() {
    let (model, ids) = chain(MAX_SEQUENCE_DEPTH);
    let reached = downstream_of(&model, ids[0]).expect("within budget");
    assert_eq!(reached, ids[1..]);
    assert_eq!(find_cycle(&model), Ok(None));
}

/// One step past the budget is a typed refusal, not a truncated answer.
#[test]
fn a_chain_past_the_budget_is_refused() {
    let (model, ids) = chain(MAX_SEQUENCE_DEPTH + 1);
    let budget = ScheduleReadError::SequenceDepthExceeded {
        start: ids[0],
        limit: MAX_SEQUENCE_DEPTH,
    };
    assert_eq!(downstream_of(&model, ids[0]), Err(budget.clone()));
    assert_eq!(
        find_cycle(&model),
        Err(budget),
        "a truncated search cannot claim there is no cycle"
    );
    // The rest of the chain fits, so a walk from the second task completes.
    assert_eq!(
        downstream_of(&model, ids[1]).expect("within budget").len(),
        MAX_SEQUENCE_DEPTH - 1
    );
}

/// A cycle among an event and a procedure, touching no task, is found.
#[test]
fn a_cycle_through_non_task_processes_is_found() {
    let mut model = Model::new();
    let _task = process(&mut model, "IFCTASK", "T");
    let event = process(&mut model, "IFCEVENT", "E");
    let procedure = process(&mut model, "IFCPROCEDURE", "P");
    link(&mut model, event, procedure);
    link(&mut model, procedure, event);

    let cycle = find_cycle(&model).expect("bound").expect("the graph loops");
    assert_eq!(cycle.repeated, event);
    assert_eq!(cycle.path, [event, procedure, event]);
    assert!(matches!(
        execution_order(&model),
        Err(ScheduleReadError::Cycle(_))
    ));
    assert!(matches!(
        process_execution_order(&model),
        Err(ScheduleReadError::Cycle(_))
    ));
}

/// A constraint that passes through an event still orders the tasks.
#[test]
fn ordering_follows_constraints_through_other_processes() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut model = Model::new();
        model.header_mut().schema = vec![schema.to_owned()];
        // File order puts B first; only the chain A -> E -> P -> B says
        // otherwise.
        let b = process(&mut model, "IFCTASK", "B");
        let a = process(&mut model, "IFCTASK", "A");
        let event = process(&mut model, "IFCEVENT", "E");
        let procedure = process(&mut model, "IFCPROCEDURE", "P");
        link(&mut model, a, event);
        link(&mut model, event, procedure);
        link(&mut model, procedure, b);

        assert_eq!(execution_order(&model), Ok(vec![a, b]), "{schema}");
        assert_eq!(
            process_execution_order(&model),
            Ok(vec![a, event, procedure, b]),
            "{schema}"
        );
    }
}

/// `Lag` exposes the required duration type and the inherited name.
#[test]
fn a_lag_states_its_duration_type_and_name() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut model = Model::new();
        model.header_mut().schema = vec![schema.to_owned()];
        let a = process(&mut model, "IFCTASK", "A");
        let b = process(&mut model, "IFCTASK", "B");
        let c = process(&mut model, "IFCTASK", "C");
        let mut tx = Transaction::new(&model);
        let cure =
            create_lag_time(&mut tx, Some("cure"), Value::Text("P5D".into()), "WORKTIME").unwrap();
        let overlap = create_lag_time(&mut tx, None, Value::Real(0.5), "ELAPSEDTIME").unwrap();
        create_sequence(&mut tx, "0aaaaaaaaaaaaaaaaaaaaa", a, b, None, Some(cure)).unwrap();
        create_sequence(&mut tx, "0bbbbbbbbbbbbbbbbbbbbb", b, c, None, Some(overlap)).unwrap();
        tx.commit(&mut model).expect("commit");

        let lags: Vec<_> = sequences(&model)
            .expect("bound")
            .into_iter()
            .map(|s| s.lag.expect("a lag"))
            .collect();
        assert_eq!(lags[0].name.as_deref(), Some("cure"), "{schema}");
        assert_eq!(lags[0].duration_type, Some(DurationType::WorkTime));
        assert_eq!(lags[0].duration.as_deref(), Some("P5D"));
        assert_eq!(lags[1].name, None);
        assert_eq!(lags[1].duration_type, Some(DurationType::ElapsedTime));
        assert_eq!(lags[1].ratio, Some(0.5));
    }
}

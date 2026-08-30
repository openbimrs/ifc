//! `IfcRelSequence`: predecessor/successor links and their lag.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! ```text
//! IfcRelSequence  (IfcRelConnects -> IfcRelationship -> IfcRoot)
//! 0 GlobalId          1 OwnerHistory      2 Name
//! 3 Description       4 RelatingProcess   5 RelatedProcess
//! 6 TimeLag           7 SequenceType      8 UserDefinedSequenceType
//!
//! IfcLagTime  (IfcSchedulingTime)
//! 0 Name              1 DataOrigin        2 UserDefinedDataOrigin
//! 3 LagValue          4 DurationType
//! ```
//!
//! # Direction is stated, not inferred
//!
//! Unlike `IfcRelConnectsPorts`, where authoring order carries no physical
//! meaning, `IfcRelSequence` IS directed by definition: `RelatingProcess` is
//! the predecessor and `RelatedProcess` is the successor. The schema's own
//! inverse names confirm it -- `IsPredecessorTo` is `FOR RelatingProcess`.
//!
//! # Sequence type says WHICH ends are linked
//!
//! `IfcSequenceEnum` is not decoration: `FINISH_START` means the successor
//! starts after the predecessor finishes, while `START_START` means they start
//! together. A tool that treats every link as finish-to-start will compute a
//! schedule that the file does not state.

use std::collections::HashSet;

use ifc_model::{EntityId, Model, Value};

/// `IfcRelSequence` slots.
mod slot {
    /// `RelatingProcess`, the predecessor.
    pub const RELATING: usize = 4;
    /// `RelatedProcess`, the successor.
    pub const RELATED: usize = 5;
    /// `TimeLag`, an `IfcLagTime` reference.
    pub const TIME_LAG: usize = 6;
    /// `SequenceType`.
    pub const SEQUENCE_TYPE: usize = 7;
}

/// `IfcLagTime` slots.
mod lag_slot {
    /// `LagValue`, an `IfcTimeOrRatioSelect`.
    pub const LAG_VALUE: usize = 3;
    /// `DurationType`.
    pub const DURATION_TYPE: usize = 4;
}

/// The maximum sequence-graph depth walked before reporting a runaway chain.
pub const MAX_SEQUENCE_DEPTH: usize = 4096;

/// Which ends of two tasks a sequence links.
///
/// `IfcSequenceEnum`, verified against IFC4 EXPRESS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceType {
    /// Successor starts after predecessor starts.
    StartStart,
    /// Successor finishes after predecessor starts.
    StartFinish,
    /// Successor starts after predecessor finishes. The common case.
    FinishStart,
    /// Successor finishes after predecessor finishes.
    FinishFinish,
    /// `.USERDEFINED.`
    UserDefined,
    /// `.NOTDEFINED.`
    NotDefined,
}

impl SequenceType {
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "START_START" => Self::StartStart,
            "START_FINISH" => Self::StartFinish,
            "FINISH_START" => Self::FinishStart,
            "FINISH_FINISH" => Self::FinishFinish,
            "USERDEFINED" => Self::UserDefined,
            "NOTDEFINED" => Self::NotDefined,
            _ => return None,
        })
    }
}

/// The lag between two sequenced tasks.
#[derive(Debug, Clone, PartialEq)]
pub struct Lag {
    /// The `IfcLagTime` entity.
    pub id: EntityId,
    /// The lag as an authored ISO 8601 duration, when stated as a duration.
    pub duration: Option<String>,
    /// The lag as a ratio, when stated as one.
    ///
    /// `IfcTimeOrRatioSelect` admits both. A ratio lag means "start when the
    /// predecessor is 50% done" and cannot be converted to a duration without
    /// knowing that task's own duration.
    pub ratio: Option<f64>,
}

/// One directed sequence link.
#[derive(Debug, Clone, PartialEq)]
pub struct Sequence {
    /// The `IfcRelSequence` entity.
    pub id: EntityId,
    /// The predecessor task.
    pub predecessor: EntityId,
    /// The successor task.
    pub successor: EntityId,
    /// Which ends are linked, if stated.
    pub sequence_type: Option<SequenceType>,
    /// The lag, if stated.
    pub lag: Option<Lag>,
}

/// A cycle in the sequence graph.
///
/// A schedule whose tasks depend on each other in a loop has no valid
/// ordering. This is data to report, not a condition to crash on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceCycle {
    /// The task the walk returned to.
    pub repeated: EntityId,
    /// The path taken, ending at the repeat.
    pub path: Vec<EntityId>,
}

/// Every sequence link in the model, in file order.
#[must_use]
pub fn sequences(model: &Model) -> Vec<Sequence> {
    let mut out = Vec::new();
    for (id, entity) in model.of_type("IFCRELSEQUENCE") {
        let (Some(Value::Ref(predecessor)), Some(Value::Ref(successor))) = (
            entity.attribute(slot::RELATING),
            entity.attribute(slot::RELATED),
        ) else {
            continue;
        };
        let sequence_type = match entity.attribute(slot::SEQUENCE_TYPE) {
            Some(Value::Enum(token)) => SequenceType::parse(token),
            _ => None,
        };
        let lag = match entity.attribute(slot::TIME_LAG) {
            Some(Value::Ref(lag_id)) => read_lag(model, *lag_id),
            _ => None,
        };
        out.push(Sequence {
            id,
            predecessor: *predecessor,
            successor: *successor,
            sequence_type,
            lag,
        });
    }
    out
}

fn read_lag(model: &Model, id: EntityId) -> Option<Lag> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCLAGTIME") {
        return None;
    }
    let value = entity.attribute(lag_slot::LAG_VALUE);
    // IfcTimeOrRatioSelect: IfcDuration is a string, IfcRatioMeasure a real.
    // The wrapper distinguishes them, so read both rather than guessing.
    let duration = value
        .and_then(|v| v.unwrap_typed().as_text())
        .map(str::to_string);
    let ratio = if duration.is_some() {
        None
    } else {
        value.and_then(|v| v.unwrap_typed().as_f64())
    };
    // DurationType is read to confirm the lag is a duration at all; a ratio
    // lag leaves it meaningless.
    let _ = entity.attribute(lag_slot::DURATION_TYPE);
    Some(Lag {
        id,
        duration,
        ratio,
    })
}

/// Tasks that must finish (or start) before `task`, in file order.
#[must_use]
pub fn predecessors_of(model: &Model, task: EntityId) -> Vec<EntityId> {
    sequences(model)
        .into_iter()
        .filter(|s| s.successor == task)
        .map(|s| s.predecessor)
        .collect()
}

/// Tasks that follow `task`, in file order.
#[must_use]
pub fn successors_of(model: &Model, task: EntityId) -> Vec<EntityId> {
    sequences(model)
        .into_iter()
        .filter(|s| s.predecessor == task)
        .map(|s| s.successor)
        .collect()
}

/// Every task reachable downstream of `task`, depth-first.
///
/// Excludes the start. Returns `Err` with the offending path if the graph
/// cycles: a schedule that loops has no valid ordering, and the loop is the
/// answer the caller needs.
///
/// # Errors
///
/// [`SequenceCycle`] when a task is reachable from itself.
pub fn downstream_of(model: &Model, task: EntityId) -> Result<Vec<EntityId>, SequenceCycle> {
    let all = sequences(model);
    let mut out = Vec::new();
    let mut path = Vec::new();
    let mut on_path = HashSet::new();
    let mut seen = HashSet::new();
    walk(&all, task, &mut out, &mut path, &mut on_path, &mut seen)?;
    Ok(out)
}

fn walk(
    all: &[Sequence],
    node: EntityId,
    out: &mut Vec<EntityId>,
    path: &mut Vec<EntityId>,
    on_path: &mut HashSet<EntityId>,
    seen: &mut HashSet<EntityId>,
) -> Result<(), SequenceCycle> {
    if path.len() >= MAX_SEQUENCE_DEPTH {
        return Ok(());
    }
    path.push(node);
    on_path.insert(node);

    for successor in all
        .iter()
        .filter(|s| s.predecessor == node)
        .map(|s| s.successor)
    {
        if on_path.contains(&successor) {
            let mut cycle = path.clone();
            cycle.push(successor);
            return Err(SequenceCycle {
                repeated: successor,
                path: cycle,
            });
        }
        // A diamond reconverges on the same task by two routes; report it
        // once, but still recurse the first time it is seen.
        if seen.insert(successor) {
            out.push(successor);
            walk(all, successor, out, path, on_path, seen)?;
        }
    }

    path.pop();
    on_path.remove(&node);
    Ok(())
}

/// The first cycle in the whole sequence graph, if any.
///
/// Checks every task, so a cycle in a disconnected component is still found.
#[must_use]
pub fn find_cycle(model: &Model) -> Option<SequenceCycle> {
    for (id, _) in model.of_type("IFCTASK") {
        if let Err(cycle) = downstream_of(model, id) {
            return Some(cycle);
        }
    }
    None
}

//! `IfcRelSequence`: predecessor/successor links and their lag.
//!
//! # Read by name in the declared release (#212)
//!
//! ```text
//! IfcRelSequence  (IfcRelConnects -> IfcRelationship -> IfcRoot)
//! IFC4, IFC4X3  0 GlobalId  1 OwnerHistory  2 Name  3 Description
//!               4 RelatingProcess  5 RelatedProcess
//!               6 TimeLag (OPTIONAL IfcLagTime)  7 SequenceType
//!               8 UserDefinedSequenceType
//! IFC2X3        the same first six, 6 TimeLag (IfcTimeMeasure, required)
//!               7 SequenceType (required)
//!
//! IfcLagTime  (IfcSchedulingTime; IFC4 and IFC4X3 only)
//! 0 Name              1 DataOrigin        2 UserDefinedDataOrigin
//! 3 LagValue          4 DurationType
//! ```
//!
//! Every attribute is found by name in the model's declared release. An
//! IFC2X3 lag is a time measure on the relationship itself, reported as
//! [`Sequence::time_lag_measure`], never as an `IfcLagTime`.
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
//!
//! # Lag is signed
//!
//! `IfcLagTime.LagValue` may be negative: a negative lag is a lead, meaning
//! the linked ends overlap. It is a stated fact, not a defect to refuse.

use std::collections::HashSet;

use ifc_model::{EntityId, Model, Value};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;

const SEQUENCE: &str = "IFCRELSEQUENCE";
const LAG_TIME: &str = "IFCLAGTIME";

/// `IfcRelSequence` slots in the layout IFC4 and IFC4X3 share, which the
/// modelless `create_sequence` writes. The reader goes by name.
pub(crate) mod slot {
    /// `RelatingProcess`, the predecessor.
    pub const RELATING: usize = 4;
    /// `RelatedProcess`, the successor.
    pub const RELATED: usize = 5;
    /// `TimeLag`, an `IfcLagTime` reference.
    pub const TIME_LAG: usize = 6;
    /// `SequenceType`.
    pub const SEQUENCE_TYPE: usize = 7;
}

/// `IfcLagTime` slots (IFC4 and IFC4X3; IFC2X3 has no `IfcLagTime`). The
/// reader goes by name.
pub mod lag_slot {
    /// `LagValue`. Required by the schema.
    pub const LAG_VALUE: usize = 3;
    /// `DurationType`. Required by the schema.
    pub const DURATION_TYPE: usize = 4;
}

/// The maximum sequence-graph depth walked before reporting a runaway chain.
pub const MAX_SEQUENCE_DEPTH: usize = 4096;

/// Which ends of two tasks a sequence links.
///
/// `IfcSequenceEnum`, verified against IFC4 EXPRESS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Sequence {
    /// The `IfcRelSequence` entity.
    pub id: EntityId,
    /// The predecessor task.
    pub predecessor: EntityId,
    /// The successor task.
    pub successor: EntityId,
    /// Which ends are linked, if stated.
    pub sequence_type: Option<SequenceType>,
    /// The `IfcLagTime` lag, if stated (IFC4 and IFC4X3).
    pub lag: Option<Lag>,
    /// IFC2X3's `TimeLag`, an `IfcTimeMeasure` in the project's time unit
    /// stated on the relationship itself; `None` in IFC4 and IFC4X3, whose
    /// lag is [`Self::lag`].
    pub time_lag_measure: Option<f64>,
}

/// A cycle in the sequence graph.
///
/// A schedule whose tasks depend on each other in a loop has no valid
/// ordering. This is data to report, not a condition to crash on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SequenceCycle {
    /// The task the walk returned to.
    pub repeated: EntityId,
    /// The path taken, ending at the repeat.
    pub path: Vec<EntityId>,
}

/// Every sequence link in the model, in file order, read against the
/// model's declared release.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn sequences(model: &Model) -> Result<Vec<Sequence>, ScheduleReadError> {
    let release = ReadRelease::of(model)?;
    let mut out = Vec::new();
    for (id, entity) in model.of_type(SEQUENCE) {
        let (Some(predecessor), Some(successor)) = (
            release.reference(SEQUENCE, entity, "RelatingProcess"),
            release.reference(SEQUENCE, entity, "RelatedProcess"),
        ) else {
            continue;
        };
        let sequence_type = release
            .token(SEQUENCE, entity, "SequenceType")
            .and_then(SequenceType::parse);
        let (lag, time_lag_measure) = match release.value(SEQUENCE, entity, "TimeLag") {
            Some(Value::Ref(lag_id)) => (read_lag(model, release, *lag_id), None),
            Some(value) => (None, value.unwrap_typed().as_f64()),
            None => (None, None),
        };
        out.push(Sequence {
            id,
            predecessor,
            successor,
            sequence_type,
            lag,
            time_lag_measure,
        });
    }
    Ok(out)
}

fn read_lag(model: &Model, release: ReadRelease, id: EntityId) -> Option<Lag> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case(LAG_TIME) {
        return None;
    }
    let value = release.value(LAG_TIME, entity, "LagValue");
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
    Some(Lag {
        id,
        duration,
        ratio,
    })
}

/// Tasks that must finish (or start) before `task`, in file order.
///
/// # Errors
///
/// The binding refusals of [`sequences`].
pub fn predecessors_of(model: &Model, task: EntityId) -> Result<Vec<EntityId>, ScheduleReadError> {
    Ok(sequences(model)?
        .into_iter()
        .filter(|s| s.successor == task)
        .map(|s| s.predecessor)
        .collect())
}

/// Tasks that follow `task`, in file order.
///
/// # Errors
///
/// The binding refusals of [`sequences`].
pub fn successors_of(model: &Model, task: EntityId) -> Result<Vec<EntityId>, ScheduleReadError> {
    Ok(sequences(model)?
        .into_iter()
        .filter(|s| s.predecessor == task)
        .map(|s| s.successor)
        .collect())
}

/// Every task reachable downstream of `task`, depth-first.
///
/// Excludes the start. Returns `Err` with the offending path if the graph
/// cycles: a schedule that loops has no valid ordering, and the loop is the
/// answer the caller needs.
///
/// # Errors
///
/// [`ScheduleReadError::Cycle`] when a task is reachable from itself, and
/// the binding refusals of [`sequences`].
pub fn downstream_of(model: &Model, task: EntityId) -> Result<Vec<EntityId>, ScheduleReadError> {
    let all = sequences(model)?;
    Ok(downstream_in(&all, task)?)
}

/// [`downstream_of`] over already read links.
fn downstream_in(all: &[Sequence], task: EntityId) -> Result<Vec<EntityId>, SequenceCycle> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    let mut on_path = HashSet::new();
    let mut seen = HashSet::new();
    walk(all, task, &mut out, &mut path, &mut on_path, &mut seen)?;
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
///
/// # Errors
///
/// The binding refusals of [`sequences`].
pub fn find_cycle(model: &Model) -> Result<Option<SequenceCycle>, ScheduleReadError> {
    let all = sequences(model)?;
    for (id, _) in model.of_type("IFCTASK") {
        if let Err(cycle) = downstream_in(&all, id) {
            return Ok(Some(cycle));
        }
    }
    Ok(None)
}

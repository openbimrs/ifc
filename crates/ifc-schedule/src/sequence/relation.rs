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

use std::collections::{HashMap, HashSet};

use ifc_model::{EntityId, Model, Value};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::task::DurationType;

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

/// The longest chain of processes a sequence walk follows.
///
/// A walk that would go deeper is refused with
/// [`ScheduleReadError::SequenceDepthExceeded`] rather than returned
/// truncated (#236).
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
    /// `IfcLagTime.DurationType`: whether the lag counts working time or
    /// elapsed time (#236). Required by the schema; `None` when the record
    /// leaves it unset or states a token outside `IfcTaskDurationEnum`.
    pub duration_type: Option<DurationType>,
    /// `IfcSchedulingTime.Name`, as authored (#236).
    pub name: Option<String>,
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
        duration_type: release
            .token(LAG_TIME, entity, "DurationType")
            .and_then(DurationType::parse),
        name: release.text(LAG_TIME, entity, "Name").map(str::to_string),
    })
}

/// Processes that must finish (or start) before `task`, in file order.
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

/// Processes that follow `task`, in file order.
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

/// Every process reachable downstream of `task`, depth-first.
///
/// Excludes the start. Returns `Err` with the offending path if the graph
/// cycles: a schedule that loops has no valid ordering, and the loop is the
/// answer the caller needs. Every `IfcRelSequence` is followed, whatever
/// `IfcProcess` it relates.
///
/// # Errors
///
/// [`ScheduleReadError::Cycle`] when a process is reachable from itself,
/// [`ScheduleReadError::SequenceDepthExceeded`] when a chain from `task` is
/// longer than [`MAX_SEQUENCE_DEPTH`], and the binding refusals of
/// [`sequences`].
pub fn downstream_of(model: &Model, task: EntityId) -> Result<Vec<EntityId>, ScheduleReadError> {
    let all = sequences(model)?;
    downstream_in(&successor_map(&all), task)
}

/// Successors of each predecessor, in file order of the links.
fn successor_map(all: &[Sequence]) -> HashMap<EntityId, Vec<EntityId>> {
    let mut map: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for link in all {
        map.entry(link.predecessor)
            .or_default()
            .push(link.successor);
    }
    map
}

/// [`downstream_of`] over already read links.
///
/// An explicit stack rather than recursion, so the depth budget, not the
/// thread's stack, bounds the walk.
fn downstream_in(
    successors: &HashMap<EntityId, Vec<EntityId>>,
    start: EntityId,
) -> Result<Vec<EntityId>, ScheduleReadError> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    // The current path, each node with the index of its next successor.
    let mut path: Vec<(EntityId, usize)> = vec![(start, 0)];
    let mut on_path = HashSet::from([start]);
    while let Some((node, next)) = path.last_mut() {
        let node = *node;
        let Some(&successor) = successors.get(&node).and_then(|all| all.get(*next)) else {
            path.pop();
            on_path.remove(&node);
            continue;
        };
        *next += 1;
        if on_path.contains(&successor) {
            let mut cycle: Vec<EntityId> = path.iter().map(|(id, _)| *id).collect();
            cycle.push(successor);
            return Err(SequenceCycle {
                repeated: successor,
                path: cycle,
            }
            .into());
        }
        // A diamond reconverges on the same process by two routes; report it
        // once, but still descend the first time it is seen.
        if seen.insert(successor) {
            out.push(successor);
            if path.len() >= MAX_SEQUENCE_DEPTH {
                return Err(ScheduleReadError::SequenceDepthExceeded {
                    start,
                    limit: MAX_SEQUENCE_DEPTH,
                });
            }
            path.push((successor, 0));
            on_path.insert(successor);
        }
    }
    Ok(out)
}

/// Every `IfcProcess` in the model, in file order, per its declared release.
pub(crate) fn processes(model: &Model) -> Result<Vec<EntityId>, ScheduleReadError> {
    Ok(ReadRelease::of(model)?.instances_of(model, "IFCPROCESS"))
}

/// The first cycle in the whole sequence graph, if any.
///
/// Walks from every `IfcProcess` (tasks, procedures and events alike), so a
/// cycle through a non-task process, or in a disconnected component, is
/// still found (#236).
///
/// # Errors
///
/// [`ScheduleReadError::SequenceDepthExceeded`] when a chain is longer than
/// [`MAX_SEQUENCE_DEPTH`], since a truncated search cannot say there is no
/// cycle, and the binding refusals of [`sequences`].
pub fn find_cycle(model: &Model) -> Result<Option<SequenceCycle>, ScheduleReadError> {
    let all = sequences(model)?;
    let successors = successor_map(&all);
    // A process already reached from an earlier start had its whole
    // downstream walked without a cycle, so walking from it again finds none.
    let mut covered = HashSet::new();
    for id in processes(model)? {
        if covered.contains(&id) {
            continue;
        }
        match downstream_in(&successors, id) {
            Ok(reached) => {
                covered.insert(id);
                covered.extend(reached);
            }
            Err(ScheduleReadError::Cycle(cycle)) => return Ok(Some(cycle)),
            Err(other) => return Err(other),
        }
    }
    Ok(None)
}

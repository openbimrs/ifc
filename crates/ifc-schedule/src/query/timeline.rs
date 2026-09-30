//! Deterministic timeline queries over an authored schedule.
//!
//! # What "deterministic" means here
//!
//! Every result is ordered explicitly -- by file order, or by a stable
//! tie-break -- so two runs over the same file produce the same sequence. That
//! matters because these results feed diffs and reports, where a hash-ordered
//! result set produces spurious changes.
//!
//! # What this module does NOT compute
//!
//! No critical path, no forward/backward pass, no date arithmetic. Those need
//! calendar expansion and a date library, which this crate does not
//! depend on. What it does provide is the ordering those algorithms run on,
//! plus the anomalies that make them meaningless if ignored.

use std::collections::{HashMap, HashSet};

use ifc_model::{EntityId, Model};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::sequence::{sequences, SequenceCycle};

const ASSIGNS: &str = "IFCRELASSIGNSTOCONTROL";
const NESTS: &str = "IFCRELNESTS";

/// `IfcRelAssignsToControl` slots, the same in IFC2X3, IFC4 and IFC4X3.
/// The writers lay records out with them; the readers go by name.
pub mod assigns {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `RelatedObjects`.
    pub const RELATED: usize = 4;
    /// `RelatingControl`.
    pub const RELATING: usize = 6;
}

/// `IfcRelNests` slots, the same in IFC2X3, IFC4 and IFC4X3. The writers
/// lay records out with them; the readers go by name.
pub mod nests {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `RelatingObject`.
    pub const RELATING: usize = 4;
    /// `RelatedObjects`.
    pub const RELATED: usize = 5;
}

/// Tasks assigned to a work schedule, in file order.
///
/// Uses `IfcRelAssignsToControl`, whose `RelatingControl` is the schedule.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn tasks_of_schedule(
    model: &Model,
    schedule: EntityId,
) -> Result<Vec<EntityId>, ScheduleReadError> {
    let release = ReadRelease::of(model)?;
    let mut out = Vec::new();
    for (_, entity) in model.of_type(ASSIGNS) {
        if release.reference(ASSIGNS, entity, "RelatingControl") != Some(schedule) {
            continue;
        }
        if let Some(v) = release.value(ASSIGNS, entity, "RelatedObjects") {
            v.for_each_ref(&mut |id| {
                if !out.contains(&id) {
                    out.push(id);
                }
            });
        }
    }
    Ok(out)
}

/// Sub-tasks nested directly under a task, in authored order.
///
/// A work breakdown structure nests tasks with `IfcRelNests`, the same
/// relationship cost items use.
///
/// # Errors
///
/// As [`tasks_of_schedule`].
pub fn subtasks_of(model: &Model, parent: EntityId) -> Result<Vec<EntityId>, ScheduleReadError> {
    let release = ReadRelease::of(model)?;
    let mut out = Vec::new();
    for (_, entity) in model.of_type(NESTS) {
        if release.reference(NESTS, entity, "RelatingObject") != Some(parent) {
            continue;
        }
        if let Some(v) = release.value(NESTS, entity, "RelatedObjects") {
            v.for_each_ref(&mut |id| out.push(id));
        }
    }
    Ok(out)
}

/// Tasks with no predecessor: where the schedule can start.
///
/// In file order, so the result is stable.
///
/// # Errors
///
/// The binding refusals of [`sequences`].
pub fn start_tasks(model: &Model) -> Result<Vec<EntityId>, ScheduleReadError> {
    let links = sequences(model)?;
    let has_predecessor: HashSet<EntityId> = links.iter().map(|s| s.successor).collect();
    Ok(model
        .ids_of_type("IFCTASK")
        .iter()
        .copied()
        .filter(|id| !has_predecessor.contains(id))
        .collect())
}

/// Tasks with no successor: where the schedule ends.
///
/// # Errors
///
/// The binding refusals of [`sequences`].
pub fn end_tasks(model: &Model) -> Result<Vec<EntityId>, ScheduleReadError> {
    let links = sequences(model)?;
    let has_successor: HashSet<EntityId> = links.iter().map(|s| s.predecessor).collect();
    Ok(model
        .ids_of_type("IFCTASK")
        .iter()
        .copied()
        .filter(|id| !has_successor.contains(id))
        .collect())
}

/// Every task in a valid execution order.
///
/// A deterministic topological sort: among tasks that are simultaneously
/// ready, the one appearing first in the file wins. Without that tie-break the
/// order would depend on hash iteration and change between runs.
///
/// The sort runs over every `IfcProcess` and keeps the `IfcTask`s (#236), so
/// a constraint that passes through an `IfcEvent` or `IfcProcedure`
/// (task A, then event E, then task B) still orders A before B, and a cycle
/// through one is still refused. [`process_execution_order`] keeps every
/// process.
///
/// # Errors
///
/// [`ScheduleReadError::Cycle`] when the graph loops, because a cyclic
/// schedule has no valid ordering at all, and the refusals of
/// [`process_execution_order`].
pub fn execution_order(model: &Model) -> Result<Vec<EntityId>, ScheduleReadError> {
    Ok(process_execution_order(model)?
        .into_iter()
        .filter(|id| {
            model
                .get(*id)
                .is_some_and(|entity| entity.type_name.eq_ignore_ascii_case("IFCTASK"))
        })
        .collect())
}

/// Every `IfcProcess` (task, procedure or event) in a valid execution order
/// (#236).
///
/// The same deterministic sort as [`execution_order`], over every process
/// the model's release declares, in file order among those ready together.
///
/// # Errors
///
/// [`ScheduleReadError::Cycle`] when the graph loops,
/// [`ScheduleReadError::SequenceDepthExceeded`] when locating that cycle
/// meets a chain longer than the walk budget, and the binding refusals of
/// [`sequences`].
pub fn process_execution_order(model: &Model) -> Result<Vec<EntityId>, ScheduleReadError> {
    let links = sequences(model)?;
    let processes = crate::sequence::relation::processes(model)?;
    let position: HashMap<EntityId, usize> = processes
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    let mut indegree: HashMap<EntityId, usize> = processes.iter().map(|id| (*id, 0)).collect();
    let mut edges: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for link in &links {
        // Ignore links naming entities that are not processes in this model:
        // a dangling sequence must not silently drop a real process.
        if !position.contains_key(&link.predecessor) || !position.contains_key(&link.successor) {
            continue;
        }
        edges
            .entry(link.predecessor)
            .or_default()
            .push(link.successor);
        *indegree.entry(link.successor).or_insert(0) += 1;
    }

    // Ready set kept sorted by file position: deterministic, and cheap at the
    // sizes real schedules reach.
    let mut ready: Vec<EntityId> = processes
        .iter()
        .copied()
        .filter(|id| indegree.get(id).copied().unwrap_or(0) == 0)
        .collect();
    ready.sort_by_key(|id| position.get(id).copied().unwrap_or(usize::MAX));

    let mut out = Vec::new();
    while let Some(next) = ready.first().copied() {
        ready.remove(0);
        out.push(next);
        for successor in edges.get(&next).cloned().unwrap_or_default() {
            let degree = indegree.entry(successor).or_insert(0);
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.push(successor);
                ready.sort_by_key(|id| position.get(id).copied().unwrap_or(usize::MAX));
            }
        }
    }

    if out.len() != processes.len() {
        // Kahn's algorithm stalls exactly when a cycle remains. Find it and
        // report the path rather than a bare "graph is cyclic".
        if let Some(cycle) = crate::sequence::find_cycle(model)? {
            return Err(cycle.into());
        }
        // Unreachable for a well-formed model: a stall implies a cycle. Report
        // the first unemitted process rather than claiming a clean result.
        let emitted: HashSet<EntityId> = out.iter().copied().collect();
        let stalled = processes
            .iter()
            .find(|id| !emitted.contains(id))
            .copied()
            .unwrap_or(EntityId(0));
        return Err(SequenceCycle {
            repeated: stalled,
            path: vec![stalled],
        }
        .into());
    }
    Ok(out)
}

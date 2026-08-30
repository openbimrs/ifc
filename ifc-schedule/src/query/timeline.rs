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
//! calendar expansion and a date library; this crate's only dependency is
//! `ifc-model`. What it does provide is the ordering those algorithms run on,
//! plus the anomalies that make them meaningless if ignored.

use std::collections::{HashMap, HashSet};

use ifc_model::{EntityId, Model, Value};

use crate::sequence::{sequences, SequenceCycle};

/// `IfcRelAssignsToControl` slots.
mod assigns {
    /// `RelatedObjects`.
    pub const RELATED: usize = 4;
    /// `RelatingControl`.
    pub const RELATING: usize = 6;
}

/// `IfcRelNests` slots.
mod nests {
    /// `RelatingObject`.
    pub const RELATING: usize = 4;
    /// `RelatedObjects`.
    pub const RELATED: usize = 5;
}

/// Tasks assigned to a work schedule, in file order.
///
/// Uses `IfcRelAssignsToControl`, whose `RelatingControl` is the schedule.
#[must_use]
pub fn tasks_of_schedule(model: &Model, schedule: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELASSIGNSTOCONTROL") {
        let relating = match entity.attribute(assigns::RELATING) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if relating != schedule {
            continue;
        }
        if let Some(v) = entity.attribute(assigns::RELATED) {
            v.for_each_ref(&mut |id| {
                if !out.contains(&id) {
                    out.push(id);
                }
            });
        }
    }
    out
}

/// Sub-tasks nested directly under a task, in authored order.
///
/// A work breakdown structure nests tasks with `IfcRelNests`, the same
/// relationship cost items use.
#[must_use]
pub fn subtasks_of(model: &Model, parent: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (_, entity) in model.of_type("IFCRELNESTS") {
        let relating = match entity.attribute(nests::RELATING) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if relating != parent {
            continue;
        }
        if let Some(v) = entity.attribute(nests::RELATED) {
            v.for_each_ref(&mut |id| out.push(id));
        }
    }
    out
}

/// Tasks with no predecessor: where the schedule can start.
///
/// In file order, so the result is stable.
#[must_use]
pub fn start_tasks(model: &Model) -> Vec<EntityId> {
    let links = sequences(model);
    let has_predecessor: HashSet<EntityId> = links.iter().map(|s| s.successor).collect();
    model
        .ids_of_type("IFCTASK")
        .iter()
        .copied()
        .filter(|id| !has_predecessor.contains(id))
        .collect()
}

/// Tasks with no successor: where the schedule ends.
#[must_use]
pub fn end_tasks(model: &Model) -> Vec<EntityId> {
    let links = sequences(model);
    let has_successor: HashSet<EntityId> = links.iter().map(|s| s.predecessor).collect();
    model
        .ids_of_type("IFCTASK")
        .iter()
        .copied()
        .filter(|id| !has_successor.contains(id))
        .collect()
}

/// Every task in a valid execution order.
///
/// A deterministic topological sort: among tasks that are simultaneously
/// ready, the one appearing first in the file wins. Without that tie-break the
/// order would depend on hash iteration and change between runs.
///
/// # Errors
///
/// [`SequenceCycle`] when the graph loops, because a cyclic schedule has no
/// valid ordering at all.
pub fn execution_order(model: &Model) -> Result<Vec<EntityId>, SequenceCycle> {
    let links = sequences(model);
    let tasks: Vec<EntityId> = model.ids_of_type("IFCTASK").to_vec();
    let position: HashMap<EntityId, usize> =
        tasks.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    let mut indegree: HashMap<EntityId, usize> = tasks.iter().map(|id| (*id, 0)).collect();
    let mut edges: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for link in &links {
        // Ignore links naming entities that are not tasks in this model: a
        // dangling sequence must not silently drop a real task from the order.
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
    let mut ready: Vec<EntityId> = tasks
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

    if out.len() != tasks.len() {
        // Kahn's algorithm stalls exactly when a cycle remains. Find it and
        // report the path rather than a bare "graph is cyclic".
        if let Some(cycle) = crate::sequence::find_cycle(model) {
            return Err(cycle);
        }
        // Unreachable for a well-formed model: a stall implies a cycle. Report
        // the first unemitted task rather than claiming a clean result.
        let stalled = tasks
            .iter()
            .find(|id| !out.contains(id))
            .copied()
            .unwrap_or(EntityId(0));
        return Err(SequenceCycle {
            repeated: stalled,
            path: vec![stalled],
        });
    }
    Ok(out)
}

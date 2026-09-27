//! Bounded breadth-first walk.
//!
//! Breadth-first is the right default for a containment tree: it yields a
//! storey's own elements before descending into nested spaces, which is the
//! order a consumer building a level-by-level view wants.

use std::collections::VecDeque;

use ahash::AHashSet;

use super::budget::{Budget, Stop, Walk};
use crate::value::EntityId;

/// Walk breadth-first from `start`, following `successors`.
///
/// See [`super::dfs::depth_first`] for why successors are supplied by the
/// caller rather than derived here.
pub fn breadth_first(
    start: EntityId,
    budget: Budget,
    mut successors: impl FnMut(EntityId) -> Vec<EntityId>,
) -> Walk {
    let mut visited = Vec::new();
    let mut seen = AHashSet::new();
    let mut revisited = Vec::new();
    let mut queue = VecDeque::from([(start, 0usize)]);
    let mut stop = Stop::Exhausted;

    while let Some((node, depth)) = queue.pop_front() {
        if !seen.insert(node) {
            revisited.push(node);
            continue;
        }
        if visited.len() >= budget.max_nodes {
            stop = Stop::NodeLimit;
            break;
        }
        visited.push(node);

        if depth >= budget.max_depth {
            stop = Stop::DepthLimit;
            continue;
        }
        for successor in successors(node) {
            queue.push_back((successor, depth + 1));
        }
    }

    revisited.sort_unstable();
    revisited.dedup();
    Walk {
        visited,
        stop,
        revisited,
    }
}

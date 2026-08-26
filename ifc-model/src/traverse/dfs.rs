//! Bounded depth-first walk.
//!
//! Order is deterministic: successors are followed in the order the edge
//! function yields them, so two runs over the same model agree. That matters
//! for tests and for diffing two traversals of the same file.

use ahash::AHashSet;

use super::budget::{Budget, Stop, Walk};
use crate::value::EntityId;

/// Walk depth-first from `start`, following `successors`.
///
/// `successors` returns the entities reachable in one step. It is a closure
/// rather than a fixed rule because the model does not know which references
/// are meaningful -- containment, aggregation and voiding are all references,
/// and only a domain crate can tell them apart.
pub fn depth_first(
    start: EntityId,
    budget: Budget,
    mut successors: impl FnMut(EntityId) -> Vec<EntityId>,
) -> Walk {
    let mut visited = Vec::new();
    let mut seen = AHashSet::new();
    let mut revisited = Vec::new();
    // (node, depth); depth is carried so the limit counts edges from the
    // start rather than stack size.
    let mut stack = vec![(start, 0usize)];
    let mut stop = Stop::Exhausted;

    while let Some((node, depth)) = stack.pop() {
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
            // Record that the walk was cut short, but keep draining the stack:
            // siblings at an admissible depth are still in scope.
            stop = Stop::DepthLimit;
            continue;
        }
        // Reversed so the first successor is popped first, making the visit
        // order match the natural reading order of the edge list.
        let next = successors(node);
        for successor in next.into_iter().rev() {
            stack.push((successor, depth + 1));
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

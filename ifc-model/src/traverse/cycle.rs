//! Cycle detection with the offending path.
//!
//! [`super::budget::Walk::revisited`] answers *whether* a graph is cyclic. When
//! a file is malformed the caller usually needs to know *where*, so this
//! reports the path that closes the loop.

use ahash::AHashSet;

use crate::value::EntityId;

/// Find one cycle reachable from `start`, if any.
///
/// Returns the path from the first repeated entity back to itself, so the
/// caller can name the entities involved in a diagnostic. Bounded by `max_depth`
/// so a pathological file cannot spin.
pub fn find_cycle(
    start: EntityId,
    max_depth: usize,
    mut successors: impl FnMut(EntityId) -> Vec<EntityId>,
) -> Option<Vec<EntityId>> {
    // Iterative DFS carrying the path, so the cycle can be sliced out of it
    // without a recursive walk that could itself overflow on a deep file.
    let mut stack: Vec<(EntityId, Vec<EntityId>)> = vec![(start, Vec::new())];

    while let Some((node, mut path)) = stack.pop() {
        if let Some(position) = path.iter().position(|seen| *seen == node) {
            let mut cycle = path[position..].to_vec();
            cycle.push(node);
            return Some(cycle);
        }
        if path.len() >= max_depth {
            continue;
        }
        path.push(node);
        // Deduplicate successors so a doubled edge is not mistaken for a cycle.
        let mut unique = AHashSet::new();
        for successor in successors(node) {
            if unique.insert(successor) {
                stack.push((successor, path.clone()));
            }
        }
    }
    None
}

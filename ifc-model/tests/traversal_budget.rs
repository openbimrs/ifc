//! Bounded walks: order, budgets, and cycle protection.
//!
//! The point of these primitives is that a malformed file cannot hang a
//! consumer. Every test here either pins a deterministic order or proves a
//! pathological graph terminates with a diagnosis.

use ahash::AHashMap;
use ifc_model::{breadth_first, depth_first, find_cycle, Budget, EntityId, Stop};

/// Edge function over a fixed adjacency list.
fn edges(pairs: &[(u64, &[u64])]) -> impl FnMut(EntityId) -> Vec<EntityId> {
    let map: AHashMap<u64, Vec<EntityId>> = pairs
        .iter()
        .map(|(from, to)| (*from, to.iter().map(|id| EntityId(*id)).collect()))
        .collect();
    move |id: EntityId| map.get(&id.0).cloned().unwrap_or_default()
}

const TREE: &[(u64, &[u64])] = &[(1, &[2, 3]), (2, &[4, 5]), (3, &[6])];

#[test]
fn depth_first_order_is_deterministic_and_follows_the_edge_order() {
    let walk = depth_first(EntityId(1), Budget::DEFAULT, edges(TREE));
    assert_eq!(
        walk.visited,
        [1, 2, 4, 5, 3, 6].map(EntityId),
        "first successor is explored first"
    );
    assert_eq!(walk.stop, Stop::Exhausted);
    assert!(walk.revisited.is_empty());
}

#[test]
fn breadth_first_yields_a_level_at_a_time() {
    let walk = breadth_first(EntityId(1), Budget::DEFAULT, edges(TREE));
    assert_eq!(walk.visited, [1, 2, 3, 4, 5, 6].map(EntityId));
    assert!(walk.stop.is_complete());
}

#[test]
fn a_cycle_terminates_and_is_reported() {
    let cyclic: &[(u64, &[u64])] = &[(1, &[2]), (2, &[3]), (3, &[1])];
    let walk = depth_first(EntityId(1), Budget::DEFAULT, edges(cyclic));

    assert_eq!(walk.visited, [1, 2, 3].map(EntityId), "each node once");
    assert_eq!(walk.revisited, [EntityId(1)], "the loop is reported");
    assert!(walk.stop.is_complete());
}

#[test]
fn the_depth_limit_truncates_and_says_so() {
    let deep: &[(u64, &[u64])] = &[(1, &[2]), (2, &[3]), (3, &[4]), (4, &[5])];
    let walk = depth_first(EntityId(1), Budget::with_depth(2), edges(deep));

    assert_eq!(
        walk.visited,
        [1, 2, 3].map(EntityId),
        "two edges from the start"
    );
    assert_eq!(walk.stop, Stop::DepthLimit);
    assert!(!walk.stop.is_complete(), "partial results are flagged");
}

#[test]
fn the_node_limit_truncates_and_says_so() {
    let budget = Budget {
        max_depth: 64,
        max_nodes: 3,
    };
    let walk = breadth_first(EntityId(1), budget, edges(TREE));

    assert_eq!(walk.visited.len(), 3);
    assert_eq!(walk.stop, Stop::NodeLimit);
}

/// A depth cut on one branch must not discard siblings at a legal depth.
#[test]
fn a_depth_cut_does_not_drop_shallow_siblings() {
    let lopsided: &[(u64, &[u64])] = &[(1, &[2, 9]), (2, &[3]), (3, &[4])];
    let walk = depth_first(EntityId(1), Budget::with_depth(1), edges(lopsided));

    assert!(walk.visited.contains(&EntityId(9)), "{:?}", walk.visited);
    assert_eq!(walk.stop, Stop::DepthLimit);
}

#[test]
fn find_cycle_returns_the_offending_path() {
    let cyclic: &[(u64, &[u64])] = &[(1, &[2]), (2, &[3]), (3, &[2])];
    let cycle = find_cycle(EntityId(1), 64, edges(cyclic)).expect("a cycle exists");

    assert_eq!(cycle.first(), cycle.last(), "the path closes: {cycle:?}");
    assert_eq!(cycle, [2, 3, 2].map(EntityId));
}

#[test]
fn find_cycle_returns_none_for_a_tree() {
    assert!(find_cycle(EntityId(1), 64, edges(TREE)).is_none());
}

/// A node listed twice as a successor is not a cycle.
#[test]
fn a_doubled_edge_is_not_a_cycle() {
    let doubled: &[(u64, &[u64])] = &[(1, &[2, 2])];
    assert!(find_cycle(EntityId(1), 64, edges(doubled)).is_none());
}

//! Graph walks with cycle protection.
//!
//! Depth-first and breadth-first walks over the relationship graph, bounded
//! against the cycles that occur in malformed files.
//!
//! Edges are supplied by the caller. This crate stores references without
//! interpreting them, so it cannot know whether a given reference means
//! containment, aggregation or voiding -- see `../AGENTS.md`. A domain crate
//! decides which edges to follow and reuses the budgets and cycle reporting
//! here.
//!
//! ## Internal split
//!
//! - `budget.rs`: shared traversal limits and reports.
//! - `dfs.rs`: deterministic depth-first traversal.
//! - `bfs.rs`: deterministic breadth-first traversal.
//! - `cycle.rs`: cycle and path diagnostics.

mod bfs;
mod budget;
mod cycle;
mod dfs;

pub use bfs::breadth_first;
pub use budget::{Budget, Stop, Walk};
pub use cycle::find_cycle;
pub use dfs::depth_first;

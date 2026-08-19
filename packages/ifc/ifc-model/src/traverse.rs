//! Graph walks with cycle protection.
//!
//! Depth-first and breadth-first walks over the relationship graph, bounded
//! against the cycles that occur in malformed files.
//!
//! Implementation is tracked in `../PLAN.md`.

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

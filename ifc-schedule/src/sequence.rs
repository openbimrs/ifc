//! `IfcRelSequence`: predecessors, successors and lag.
//!
//! Finish-to-start and friends, with `IfcLagTime`. Cycle detection matters --
//! a cyclic schedule must be reported, not looped over.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `relation.rs`: IfcRelSequence.
//! - `lag.rs`: lag values.
//! - `graph.rs`: bounded DAG/cycle reporting.

mod graph;
mod lag;
mod relation;

//! `IfcRelConnectsPorts` and network traversal.
//!
//! The graph walk that answers 'what is downstream of this valve'. Cycles are
//! legal here (ring mains), so traversal must handle them by design.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `relation.rs`: port/element connections.
//! - `graph.rs`: semantic graph.
//! - `traversal.rs`: bounded traversal.

mod graph;
mod relation;
mod traversal;

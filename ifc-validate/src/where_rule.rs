//! EXPRESS `WHERE` rules and the 2 global rules in IFC4.
//!
//! IFC4 declares 47 functions and 2 global rules. These are the expensive
//! checks, so they are opt-in rather than part of a default validation pass.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `registry.rs`: explicit support-state registry.
//! - `engine.rs`: bounded rule invocation.
//! - `budget.rs`: rule execution limits.
//! - `builtin.rs`: implemented generic rules.

mod budget;
mod builtin;
mod engine;
mod registry;

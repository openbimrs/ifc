//! EXPRESS `WHERE` rules, and honesty about the ones not evaluated.
//!
//! IFC4 declares hundreds of rules as EXPRESS expressions plus 2 global
//! rules. This crate has no expression evaluator, so it implements the rules
//! that are checkable from structure and *reports* the rest as unsupported
//! rather than skipping them silently.
//!
//! ## Internal split
//!
//! - `registry.rs`: explicit support-state registry.
//! - `engine.rs`: bounded rule invocation.
//! - `budget.rs`: rule execution limits.
//! - `builtin.rs`: implemented native rules.

mod budget;
mod builtin;
mod engine;
mod registry;

pub use budget::Budget;
pub use engine::evaluate;
pub use registry::{implemented, lookup, unsupported, RuleEntry, Support, RULES};

//! EXPRESS `WHERE` rules, and honesty about the ones not evaluated.
//!
//! The bundled IFC schemas declare hundreds of rules. This crate has no
//! general expression evaluator, so it implements selected predicates that
//! are provable from direct structure/scalars and *reports* known unsupported
//! rules rather than skipping them silently.
//!
//! ## Internal split
//!
//! - `registry.rs`: explicit support-state registry.
//! - `engine.rs`: bounded rule invocation.
//! - `budget.rs`: rule execution limits.
//! - `builtin.rs`: implemented native rules.
//! - `operand.rs`: reading a rule's operands, and reporting the ones that
//!   cannot be read as evaluation errors instead of skipping them.

mod budget;
mod builtin;
mod engine;
mod operand;
mod registry;

pub use budget::Budget;
pub use engine::evaluate;
pub use registry::{implemented, lookup, unsupported, RuleEntry, Support, RULES};

//! Structural conformance: references, slots, cardinality, uniqueness.
//!
//! ## Internals
//!
//! - `reference`: dangling and wrong-kind references
//! - `required`: required/derived slot presence and record arity
//! - `cardinality`: scalar-vs-aggregate shape
//! - `bounds`: aggregate bounds, nesting and element uniqueness
//! - `unique`: the release's `UNIQUE` clauses

mod bounds;
mod cardinality;
mod reference;
mod required;
mod unique;

pub use bounds::aggregate_bounds;
pub use cardinality::aggregate_shape;
pub(crate) use reference::expected_references;
pub use reference::{dangling_references, wrong_kind_references};
pub use required::required_attributes;
pub use unique::unique_rules;

use ifc_model::Model;
use ifc_schema::Schema;

use crate::report::Report;
use crate::where_rule::Budget;

/// Every structural check, in a fixed order.
///
/// Order is fixed so two runs over the same file produce identical reports;
/// the report sorts findings anyway, but a stable production order keeps
/// truncation deterministic when the budget is hit.
pub fn check(model: &Model, schema: &Schema, budget: Budget, report: &mut Report) {
    dangling_references(model, report);
    wrong_kind_references(model, schema, report);
    required_attributes(model, schema, report);
    aggregate_shape(model, schema, report);
    aggregate_bounds(model, schema, report);
    unique_rules(model, schema, report);
    if report.findings().len() >= budget.max_findings {
        report.mark_truncated();
    }
}

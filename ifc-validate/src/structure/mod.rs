//! Structural conformance: references, slots, cardinality, uniqueness.
//!
//! ## Internals
//!
//! - `reference`: dangling and wrong-kind references
//! - `required`: required/derived slot presence and record arity
//! - `cardinality`: scalar-vs-aggregate shape
//! - `unique`: duplicate `GlobalId`s

mod cardinality;
mod reference;
mod required;
mod unique;

pub use cardinality::aggregate_shape;
pub use reference::{dangling_references, wrong_kind_references};
pub use required::required_attributes;
pub use unique::duplicate_global_ids;

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
    duplicate_global_ids(model, schema, report);
    if report.findings().len() >= budget.max_findings {
        report.mark_truncated();
    }
}

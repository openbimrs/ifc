//! Type conformance: entity types, scalars, enumerations, selects.
//!
//! ## Internals
//!
//! - `scalar`: EXPRESS primitives and the forms they accept
//! - `enumeration`: enumeration membership
//! - `select`: SELECT membership as a graph walk
//! - `defined`: one value against one declared type
//! - `entity`: the per-entity sweep that reports findings

mod defined;
mod entity;
mod enumeration;
mod scalar;
mod select;

pub use defined::{check as check_value, Mismatch};
pub use entity::{abstract_instances, attribute_types, unknown_entity_types};
pub use scalar::Primitive;

use ifc_model::Model;
use ifc_schema::Schema;

use crate::report::Report;
use crate::where_rule::Budget;

/// Every type check, in a fixed order.
pub fn check(model: &Model, schema: &Schema, budget: Budget, report: &mut Report) {
    unknown_entity_types(model, schema, report);
    abstract_instances(model, schema, report);
    attribute_types(model, schema, report);
    if report.findings().len() >= budget.max_findings {
        report.mark_truncated();
    }
}

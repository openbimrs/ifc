//! Bounded IFC4 metric, objective, and constraint-relationship semantics.
//!
//! Values are projected and preserved; this crate does not evaluate compliance,
//! formulas, references, tables, or time series.
#![deny(missing_docs)]

mod authoring;
mod error;
mod projection;
mod types;
mod view;

pub use authoring::{
    associate_constraint, create_metric, create_objective, relate_resource_constraint,
    ConstraintAssociationDraft, ConstraintBaseDraft, MetricDraft, ObjectiveDraft,
    ResourceConstraintDraft,
};
pub use error::{ConstraintError, ConstraintResult};
pub use projection::{ConstraintAssignment, Metric, Objective, ResourceConstraintRelationship};
pub use types::{
    Benchmark, ConstraintGrade, LogicalOperator, MetricValue, MetricValueDraft, ObjectiveQualifier,
};
pub use view::ConstraintView;

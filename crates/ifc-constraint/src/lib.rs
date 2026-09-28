//! Bounded IFC4 metric, objective, and constraint-relationship semantics.
//!
//! Values are projected and preserved; this crate does not evaluate compliance,
//! formulas, references, tables, or time series.

mod association;
mod authoring;
mod error;
mod projection;
mod release;
mod types;
mod view;

pub use association::{
    associate_constraint, associate_constraint_with_owner_history, ConstraintAssociationDraft,
};
pub use authoring::{
    create_metric, create_objective, create_reference, relate_resource_constraint,
    ConstraintBaseDraft, MetricDraft, ObjectiveDraft, ReferenceDraft, ResourceConstraintDraft,
};
pub use error::{ConstraintError, ConstraintResult};
pub use projection::{ConstraintAssignment, Metric, Objective, ResourceConstraintRelationship};
pub use types::{
    Benchmark, ConstraintGrade, LogicalOperator, MetricValue, MetricValueDraft, ObjectiveQualifier,
};
pub use view::ConstraintView;

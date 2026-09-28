//! Bounded metric, objective, and constraint-relationship semantics, read and
//! written by attribute name in the model's declared release (IFC2X3, IFC4
//! or IFC4X3).
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
    ConstraintBaseDraft, DateTimeInput, MetricDraft, ObjectiveDraft, ReferenceDraft,
    ResourceConstraintDraft,
};
pub use error::{ConstraintError, ConstraintResult};
/// The IFC release a projection reads against (re-exported from
/// `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use projection::{ConstraintAssignment, Metric, Objective, ResourceConstraintRelationship};
pub use types::{
    Benchmark, ConstraintGrade, LogicalOperator, MetricValue, MetricValueDraft, ObjectiveQualifier,
};
pub use view::ConstraintView;

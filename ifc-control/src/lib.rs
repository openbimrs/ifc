//! Bounded IFC control semantics: permits, project orders, action
//! requests, and performance history.
//!
//! This crate owns the `IfcControl` subtypes that govern work rather
//! than describe physical form. It stages them into a transaction and
//! validates what the schema states; it does not implement approval
//! workflow, authorization, or policy decisions, which are the
//! concern of whatever system issues the permit.
//!
//! Controls relate to the work they govern through
//! `IfcRelAssignsToControl`. The crate owning the relating control
//! writes that relationship, so this crate stages it for its own four
//! controls and refuses any other.

mod assignment;
mod authoring;
mod error;

pub use assignment::{assign_to_control, ControlAssignmentDraft};
pub use authoring::{create_control, ControlDraft, ControlKind};
pub use error::{ControlError, ControlResult};

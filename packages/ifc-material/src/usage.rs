//! Resolving which material applies to a given element.
//!
//! Material can be assigned to the element or to its type, with the element
//! winning. Resolution order is a common source of wrong answers.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `assignment.rs`: RelAssociatesMaterial view.
//! - `resolution.rs`: bounded association resolution.

mod assignment;
mod ifc4_type_objects;
mod resolution;

pub use assignment::MaterialAssignment;
pub use resolution::{
    AssignmentSource, MaterialDefinition, MaterialUsageDefinition, ResolvedAssignment,
    ResolvedMaterialSelect,
};

//! Resolving which material applies to a given element.
//!
//! Material can be assigned to the element or to its type, with the element
//! winning. Resolution order is a common source of wrong answers.

//! ## Internal split
//!
//! - `assignment.rs`: RelAssociatesMaterial view.
//! - `resolution.rs`: bounded association resolution.

mod assignment;
mod resolution;

pub use assignment::MaterialAssignment;
#[cfg(test)]
pub(crate) use resolution::SELECT_MEMBERS;
pub use resolution::{
    AssignmentSource, MaterialDefinition, MaterialUsageDefinition, ResolvedAssignment,
    ResolvedMaterialSelect,
};

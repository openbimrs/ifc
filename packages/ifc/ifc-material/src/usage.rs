//! Resolving which material applies to a given element.
//!
//! Material can be assigned to the element or to its type, with the element
//! winning. Resolution order is a common source of wrong answers.
//!
//! Implementation is tracked in the adjacent `PLAN.md`.

//! ## Internal split
//!
//! - `assignment.rs`: RelAssociatesMaterial view.
//! - `resolution.rs`: bounded association resolution.

mod assignment;
mod resolution;

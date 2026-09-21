//! Element, resource, and process type definitions.
//!
//! An IFC model separates what a thing *is* from where it sits. The
//! type definition carries the shared description: a pump type names
//! the pump model, and every pump placed from it inherits that.
//!
//! This crate owns the 132 concrete `IfcTypeObject` subtypes. They
//! sit in one crate because they are one uniform shape with one
//! shared WHERE rule, not because they are one domain: the list
//! spans HVAC, structure, furniture, resources and processes.
//! Splitting them across domain crates would copy the same rule and
//! the same slot table into a dozen places.
//!
//! The catalogue in [`table`] is generated from the schema by
//! `scripts/gen-element-types.py`; [`create_type`] is the writer.

mod authoring;
pub mod table;

pub use authoring::{create_type, ElementTypeError, ElementTypeResult, Slot6, TypeDraft};
pub use table::{ElementType, Family, ALL};

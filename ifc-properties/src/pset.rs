//! `IfcPropertySet` and single/enumerated/list/table properties.
//!

//! ## Internal split
//!
//! - `set.rs`: IfcPropertySet and relationships.
//! - `scalar.rs`: single/bounded/list/enumerated values.
//! - `table.rs`: table values and interpolation metadata.
//! - `reference.rs`: object/reference properties.
//! - `complex.rs`: nested complex properties.

mod complex;
mod reference;
pub(crate) mod scalar;
mod set;
mod table;

mod aggregate;

pub use scalar::{property, Property, PropertyValue};
pub use set::{property_set, property_sets_by_object, AttachedSets, Attachment, PropertySet};

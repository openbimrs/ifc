//! `IfcPropertySet` and single/enumerated/list/table properties.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `set.rs`: IfcPropertySet and relationships.
//! - `scalar.rs`: single/bounded/list/enumerated values.
//! - `table.rs`: table values and interpolation metadata.
//! - `reference.rs`: object/reference properties.
//! - `complex.rs`: nested complex properties.

mod complex;
mod reference;
mod scalar;
mod set;
mod table;

mod aggregate;

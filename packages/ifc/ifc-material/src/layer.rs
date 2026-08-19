//! `IfcMaterialLayer` and `IfcMaterialLayerSet` semantic projections.
//!
//! Geometry-affecting usage direction, sense, offset, and reference extent are
//! owned by `ifc-geometry::input`; this module owns material composition only.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: identity, material link, and authored thickness.
//! - `set.rs`: ordered layer membership.
//! - `usage.rs`: association to a layer set, without geometry slots.

mod definition;
mod set;
mod usage;

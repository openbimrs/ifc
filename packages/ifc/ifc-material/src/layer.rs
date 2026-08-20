//! `IfcMaterialLayer` and `IfcMaterialLayerSet` semantic projections.
//!
//! This module projects the authored MaterialResource composition and usage
//! slots. `ifc-geometry::input` owns their geometric interpretation and lowering.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: identity, material link, and authored thickness.
//! - `set.rs`: ordered layer membership.
//! - `usage.rs`: authored direction, sense, offset, extent, and set association.

mod definition;
mod set;
mod usage;

pub use definition::{MaterialLayer, MaterialLayerWithOffsets};
pub use set::MaterialLayerSet;
pub use usage::MaterialLayerSetUsage;

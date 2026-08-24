//! Semantic half of `IfcMaterialProfile*`.
//!
//! This module projects authored MaterialResource profile references, cardinal
//! points, extents, offsets, and taper fields. `ifc-geometry::input` owns their
//! geometric interpretation and lowering.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: material, name, description, priority, and category.
//! - `set.rs`: ordered semantic membership and composite indicator.
//! - `usage.rs`: authored cardinal, extent, offset, and taper usage slots.

mod definition;
mod set;
mod usage;

pub use definition::{MaterialProfile, MaterialProfileWithOffsets};
pub use set::MaterialProfileSet;
pub use usage::{MaterialProfileSetUsage, MaterialProfileSetUsageTapering};

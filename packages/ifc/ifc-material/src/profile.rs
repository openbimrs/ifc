//! Semantic half of `IfcMaterialProfile*`.
//!
//! Profile references, cardinal points, reference extents, offsets, and taper
//! geometry are owned by `ifc-geometry::input`; this module owns metadata only.
//!
//! Implementation is tracked in the adjacent `PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: material, name, description, priority, and category.
//! - `set.rs`: ordered semantic membership and composite indicator.
//! - `usage.rs`: association to a profile set, without geometry slots.

mod definition;
mod set;
mod usage;

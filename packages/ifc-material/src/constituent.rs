//! `IfcMaterialConstituentSet` for non-layered composites.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: constituent semantics.
//! - `set.rs`: set membership.

mod definition;
mod set;

pub use definition::MaterialConstituent;
pub use set::MaterialConstituentSet;

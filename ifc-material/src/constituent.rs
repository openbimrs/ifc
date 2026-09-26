//! `IfcMaterialConstituentSet` for non-layered composites.
//!
//! ## Internal split
//!
//! - `definition.rs`: constituent semantics.
//! - `set.rs`: set membership.

mod definition;
mod set;

pub use definition::MaterialConstituent;
pub use set::MaterialConstituentSet;

//! `IfcMaterial` and material properties.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `definition.rs`: IfcMaterial identity.
//! - `properties.rs`: material property relationships.

//! ## Internal split
//!
//! - `relationships.rs`: material lists, classifications, and
//!   resource-level relationships.

mod definition;
mod properties;
mod relationships;

pub use definition::Material;
pub use properties::MaterialProperties;
pub use relationships::{MaterialClassificationRelationship, MaterialList, MaterialRelationship};

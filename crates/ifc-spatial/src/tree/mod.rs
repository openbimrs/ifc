//! The project → site → building → storey → element tree.
//!
//! ## Internal split
//!
//! - `kind.rs`: classifying an entity's place in the spatial hierarchy,
//!   against the declared release's schema table.
//! - `build.rs`: assembling the tree from relationship entities.
//! - `anomaly.rs`: conflicting parents and non-container structures the
//!   file states.
//! - `reference.rs`: elements referenced, not contained, by a structure.

mod anomaly;
mod build;
mod kind;
mod reference;

pub use anomaly::SpatialAnomaly;
pub use build::{SpatialNode, SpatialTree};
pub use kind::SpatialKind;

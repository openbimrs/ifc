//! The project → site → building → storey → element tree.
//!
//! ## Internal split
//!
//! - `kind.rs`: classifying an entity's place in the spatial hierarchy.
//! - `build.rs`: assembling the tree from relationship entities.
//! - `anomaly.rs`: conflicting parents the file states.

mod anomaly;
mod build;
mod kind;

pub use anomaly::SpatialAnomaly;
pub use build::{SpatialNode, SpatialTree};
pub use kind::SpatialKind;

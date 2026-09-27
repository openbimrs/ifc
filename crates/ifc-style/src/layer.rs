//! Presentation layer assignment views.
//!
//! Layer membership, identifiers, visibility flags, and layer styles remain
//! observable independently from direct `IfcStyledItem` assignments.
//!
//! Cascade resolution lives in `assignment::resolution`; this module does not
//! hide lower-priority layer evidence when a direct style wins.
//! Queries return stable entity IDs in deterministic order.

mod assignment;
mod style;

pub use assignment::PresentationLayer;

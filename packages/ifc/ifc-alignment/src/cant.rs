//! Superelevation (`IfcAlignmentCant`) for rail.
//!
//!
//! Implementation is tracked in the adjacent `PLAN.md`.

//! ## Internal split
//!
//! - `layout.rs`: cant segment order.
//! - `segment.rs`: cant transitions.

mod layout;
mod segment;

mod transition;

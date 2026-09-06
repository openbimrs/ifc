//! Superelevation (`IfcAlignmentCant`) for rail.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `layout.rs`: cant segment order.
//! - `segment.rs`: cant transitions.

mod evaluate;
mod layout;
mod segment;

mod transition;

pub use evaluate::{cant_at, CantAtStation};
pub use layout::CantLayout;
pub use segment::{read_cant_segment, CantSegment, CantSegmentType};

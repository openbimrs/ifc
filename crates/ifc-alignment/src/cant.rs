//! Superelevation (`IfcAlignmentCant`) for rail.
//!
//! ## Internal split
//!
//! - `layout.rs`: cant segment order.
//! - `segment.rs`: cant transitions.

mod evaluate;
mod layout;
mod segment;

pub use evaluate::{cant_at, CantAtStation};
pub use layout::CantLayout;
pub use segment::{read_cant_segment, CantSegment, CantSegmentType};

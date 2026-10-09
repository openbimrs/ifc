//! Superelevation (`IfcAlignmentCant`) for rail.
//!
//! ## Internal split
//!
//! - `layout.rs`: cant segment order.
//! - `segment.rs`: cant transitions.
//! - `evaluate.rs`: exact cant per segment type.
//! - `frame.rs`: cant at a station as rail heights, bank angle and frame.

mod evaluate;
mod frame;
mod layout;
mod segment;

pub use evaluate::{cant_at, CantAtStation};
pub(crate) use evaluate::{viennese_rotation, VienneseRotation};
pub use frame::CantFrame;
pub use layout::CantLayout;
pub use segment::{read_cant_segment, CantSegment, CantSegmentType};

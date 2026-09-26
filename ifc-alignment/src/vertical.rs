//! Vertical segments: grades and parabolic curves.
//!
//! ## Internal split
//!
//! - `layout.rs`: profile order.
//! - `segment.rs`: gradients/arcs/parabolas.

mod layout;
mod segment;

mod transition;

pub use segment::{read_vertical_segment, VerticalSegment, VerticalSegmentType};

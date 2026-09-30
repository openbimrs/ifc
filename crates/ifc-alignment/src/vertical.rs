//! Vertical segments: grades and parabolic curves.
//!
//! ## Internal split
//!
//! - `segment.rs`: gradients/arcs/parabolas.

mod segment;

pub use segment::{read_vertical_segment, VerticalSegment, VerticalSegmentType};

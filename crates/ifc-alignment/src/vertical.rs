//! Vertical segments: grades and parabolic curves.
//!
//! ## Internal split
//!
//! - `layout.rs`: vertical segment order.
//! - `segment.rs`: gradients/arcs/parabolas.

mod layout;
mod segment;

pub use layout::VerticalLayout;
pub use segment::{read_vertical_segment, VerticalSegment, VerticalSegmentType};

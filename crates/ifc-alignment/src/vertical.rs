//! Vertical segments: grades and parabolic curves.
//!
//! ## Internal split
//!
//! - `layout.rs`: vertical segment order.
//! - `segment.rs`: gradients/arcs/parabolas.
//! - `seam.rs`: seams between segments, and grade breaks.

mod layout;
mod seam;
mod segment;

pub use layout::VerticalLayout;
pub use seam::{VerticalSeam, VerticalSeamKind};
pub use segment::{read_vertical_segment, VerticalSegment, VerticalSegmentType};

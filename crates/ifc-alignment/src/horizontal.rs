//! Horizontal segments: line, arc, spiral transitions.
//!
//! ## Internal split
//!
//! - `segment.rs`: line/arc/transition parameters.

mod segment;

pub use segment::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};

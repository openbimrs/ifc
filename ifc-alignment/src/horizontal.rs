//! Horizontal segments: line, arc, spiral transitions.
//!
//! ## Internal split
//!
//! - `layout.rs`: segment order and continuity.
//! - `segment.rs`: line/arc/transition parameters.

mod layout;
mod segment;

mod transition;

pub use segment::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};

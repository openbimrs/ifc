//! Curve capability scaffold.

//! ## Internal split
//!
//! - `assemble.rs`: exact neutral composite curve.
//! - `elevation.rs`: vertical segments as exact elevation laws.
//! - `gradient.rs`: plan and profile composed as an exact 3D centreline.

mod assemble;
mod elevation;
mod gradient;
mod spiral;

mod provenance;
mod transition;

pub use assemble::{
    lower_horizontal_layout, lower_horizontal_layout_partial, lower_horizontal_segment,
    lower_vertical_segment, LoweredAlignmentCurve, PartialHorizontalLayout, RefusedSegment,
};
pub use elevation::{elevation_law, profile_law};
pub use gradient::lower_gradient_curve;

//! Curve capability scaffold.

//! ## Internal split
//!
//! - `assemble.rs`: exact neutral composite curve.
//! - `elevation.rs`: vertical segments as exact elevation laws.
//! - `gradient.rs`: plan and profile composed as an exact 3D centreline.
//! - `reference.rs`: the cant-carrying 3D centreline, refused until Axiolid
//!   has a roll law.
//! - `tolerance.rs`: the seam tolerance a vertical profile is checked at.

mod assemble;
mod elevation;
mod gradient;
mod reference;
mod spiral;
mod tolerance;

pub use assemble::{
    lower_horizontal_layout, lower_horizontal_layout_partial, lower_horizontal_segment,
    lower_vertical_segment, LoweredAlignmentCurve, PartialHorizontalLayout, RefusedSegment,
};
pub use elevation::{elevation_law, profile_law, profile_law_within, vertical_profile_law};
pub use gradient::{gradient_curve3, lower_gradient_curve};
pub use reference::lower_segmented_reference_curve;
pub use tolerance::SeamTolerance;

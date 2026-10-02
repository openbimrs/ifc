//! Curve capability scaffold.

//! ## Internal split
//!
//! - `assemble.rs`: exact neutral composite curve.
//! - `elevation.rs`: vertical segments as exact elevation laws.
//! - `gradient.rs`: plan and profile composed as an exact 3D centreline.
//! - `reference.rs`: the cant-carrying 3D centreline, refused until Axiolid
//!   has a roll law.
//! - `tolerance.rs`: the seam tolerance a vertical profile is checked at.
//! - `plan.rs`: a whole horizontal layout as one exact intrinsic curve.
//! - `seam.rs`: the closed-form rule for checking horizontal seams.
//! - `terminal.rs`: the zero-length segment that closes every layout.

mod assemble;
mod elevation;
mod gradient;
mod plan;
mod reference;
mod seam;
mod spiral;
pub(crate) mod terminal;
mod tolerance;

pub use assemble::{
    lower_horizontal_layout, lower_horizontal_layout_partial, lower_horizontal_segment,
    lower_vertical_segment, LoweredAlignmentCurve, PartialHorizontalLayout, RefusedSegment,
};
pub use elevation::{elevation_law, profile_law, profile_law_within, vertical_profile_law};
pub use gradient::{gradient_curve3, lower_gradient_curve};
pub use plan::{lower_horizontal_plan, HorizontalPlan};
pub use reference::lower_segmented_reference_curve;
pub use seam::{HorizontalSeam, SeamCheck};
pub use tolerance::SeamTolerance;

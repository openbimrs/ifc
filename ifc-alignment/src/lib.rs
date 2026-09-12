//! `ifc-alignment` -- Linear referencing and alignment -- the IFC4x3 civil layer.
//!
//!
//! IFC4x3 adds 14 alignment entities plus spiral curve types (`IfcClothoid`,
//! `IfcCosineSpiral`). Isolated in its own crate because building-only
//! consumers should never compile clothoid integration.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `alignment` | `IfcAlignment` and its horizontal/vertical/cant parts |
//! | `horizontal` | Horizontal segments: line, arc, spiral transitions |
//! | `vertical` | Vertical segments: grades and parabolic curves |
//! | `cant` | Superelevation (`IfcAlignmentCant`) for rail |
//! | `referent` | `IfcReferent` stationing and chainage |
//! | `placement` | `IfcLinearPlacement` and distance expressions |
//! | `error` | Why an alignment operation failed |
//!
//! Horizontal line and circular-arc parameter resolution lowers to exact
//! neutral curve graphs. Transition curves remain typed unsupported until the
//! neutral curve vocabulary can preserve them without approximation.

// Documentation debt: this crate does not yet document every public item,
// so it cannot join [workspace.lints] (missing_docs = deny) yet. The
// remaining count is budgeted in scripts/check-missing-docs.py, which fails
// if it grows. Delete this attribute once the count reaches zero and add
// `[lints] workspace = true` to Cargo.toml instead.
#![allow(missing_docs)]

mod alignment;
mod cant;
mod curve;
mod error;
mod horizontal;
mod placement;
mod referent;
mod vertical;
mod view;

pub use cant::{
    cant_at, read_cant_segment, CantAtStation, CantLayout, CantSegment, CantSegmentType,
};
pub use curve::{
    lower_horizontal_layout, lower_horizontal_layout_partial, lower_horizontal_segment,
    lower_vertical_segment, LoweredAlignmentCurve, PartialHorizontalLayout, RefusedSegment,
};
pub use error::{AlignmentError, AlignmentResult};
pub use horizontal::{
    read_horizontal_segment, AlignmentUnits, HorizontalSegment, HorizontalSegmentType,
};
pub use placement::{
    resolve_linear_placement, resolve_point_by_distance, CurveMeasure, LinearPlacement,
    PointByDistance,
};
pub use referent::{station_equations, StationEquation};
pub use vertical::{read_vertical_segment, VerticalSegment, VerticalSegmentType};
pub use view::AlignmentView;

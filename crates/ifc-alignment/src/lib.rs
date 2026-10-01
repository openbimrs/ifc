//! `ifc-alignment` -- Linear referencing and alignment -- the IFC4x3 civil layer.
//!
//!
//! IFC4x3 adds 14 alignment entities plus spiral curve types (`IfcClothoid`,
//! `IfcCosineSpiral`). Isolated in its own crate because building-only
//! consumers should never compile spiral curve laws.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `horizontal` | Horizontal segments: line, arc, spiral transitions |
//! | `vertical` | Vertical segments: grades and parabolic curves |
//! | `cant` | Superelevation (`IfcAlignmentCant`) for rail |
//! | `referent` | `IfcReferent` stationing and chainage |
//! | `placement` | `IfcLinearPlacement` and distance expressions |
//! | `error` | Why an alignment operation failed |
//!
//! Horizontal lines, circular arcs and transition spirals lower to exact
//! neutral curve graphs; a spiral is stored as its curvature law
//! (`Curve2::Intrinsic`), never integrated here. A whole layout is also one
//! intrinsic plan curve with a piecewise curvature law, which is what an
//! elevated 3D centreline carries. Families without an exact law are typed
//! refusals, tracked in GitHub issues.

pub mod authoring;
mod cant;
mod curve;
mod error;
mod horizontal;
mod placement;
mod referent;
pub(crate) mod slot;
mod vertical;
mod view;

pub use authoring::{
    alignment, alignment_segment, axis2_placement_linear, cant_layout, cant_segment,
    cartesian_point, horizontal_layout, horizontal_segment, linear_element, linear_placement,
    linear_positioning_element, point_by_distance, referent, stationing, vertical_layout,
    vertical_segment, CantSegmentDraft, HorizontalSegmentDraft, VerticalSegmentDraft,
};
pub use cant::{
    cant_at, read_cant_segment, CantAtStation, CantLayout, CantSegment, CantSegmentType,
};
pub use curve::{
    elevation_law, gradient_curve3, lower_gradient_curve, lower_horizontal_layout,
    lower_horizontal_layout_partial, lower_horizontal_plan, lower_horizontal_segment,
    lower_vertical_segment, profile_law, HorizontalPlan, HorizontalSeam, LoweredAlignmentCurve,
    PartialHorizontalLayout, RefusedSegment, SeamCheck,
};
pub use error::{AlignmentError, AlignmentResult, ProfileSeam};
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

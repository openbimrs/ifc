//! `IfcLinearPlacement` and distance expressions.
//!
//! ## Internal split
//!
//! - `linear.rs`: linear placement.

mod linear;

pub use linear::{
    resolve_linear_placement, resolve_point_by_distance, CurveMeasure, LinearPlacement,
    PointByDistance,
};

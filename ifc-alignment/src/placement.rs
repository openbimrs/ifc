//! `IfcLinearPlacement` and distance expressions.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `linear.rs`: linear placement.
//! - `distance.rs`: point-by-distance expressions.

mod distance;
mod linear;

mod station;

pub use linear::{
    resolve_linear_placement, resolve_point_by_distance, CurveMeasure, LinearPlacement,
    PointByDistance,
};

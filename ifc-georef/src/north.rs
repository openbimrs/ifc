//! True north versus project north.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `directions.rs`: true/grid/project north.

mod directions;

pub use directions::{
    grid_north_direction, project_north_direction, resolve_true_north, NorthReference,
};

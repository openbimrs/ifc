//! IFC project-to-map coordinate operations.
//!
//! This crate resolves IFC references, units, axis defaults, and CRS metadata,
//! then emits a format-neutral `axiolid_core::Transform3`. It does not place
//! products, reproject coordinates, or select a geometry backend.

// Documentation debt: this crate does not yet document every public item,
// so it cannot join [workspace.lints] (missing_docs = deny) yet. The
// remaining count is budgeted in scripts/check-missing-docs.py, which fails
// if it grows. Delete this attribute once the count reaches zero and add
// `[lints] workspace = true` to Cargo.toml instead.
#![allow(missing_docs)]

mod context;
mod conversion;
mod crs;
mod elevation;
mod error;
mod north;
mod view;

pub use context::compose_project_frame;
pub use conversion::{resolve_project_to_map, resolve_project_to_map_in, ProjectToMap};
pub use crs::{LengthUnit, ProjectedCrs};
pub use error::{GeorefError, GeorefResult};
pub use north::{
    grid_north_direction, project_north_direction, resolve_true_north, NorthReference,
};
pub use view::GeorefView;

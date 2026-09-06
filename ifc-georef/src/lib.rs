//! IFC project-to-map coordinate operations.
//!
//! This crate resolves IFC references, units, axis defaults, and CRS metadata,
//! then emits a format-neutral `axiolid_core::Transform3`. It does not place
//! products, reproject coordinates, or select a geometry backend.

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

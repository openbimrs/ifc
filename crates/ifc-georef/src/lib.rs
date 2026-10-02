//! IFC project-to-map coordinate operations.
//!
//! This crate resolves IFC references, units, axis defaults, and CRS metadata
//! into plain-number operation parameters and, with the default `transform`
//! feature, a format-neutral `axiolid_core::Transform3`. It does not place
//! products, reproject coordinates, or select a geometry backend.
//!
//! # Features
//!
//! - `transform` (default): links `axiolid-core` and adds
//!   `ProjectToMap::transform` and `compose_project_frame`. With
//!   `default-features = false` the crate links no geometry crate and still
//!   exposes every resolved parameter: CRS metadata and units, eastings,
//!   northings and height, axis direction, scale and IFC4X3 factors, the
//!   affine parts through [`ProjectToMap::map_point`], true and grid north,
//!   operation sources and the site reference.

pub mod authoring;
mod context;
mod conversion;
mod crs;
mod error;
mod north;
mod site;
mod slot;
mod view;

pub use authoring::{
    create_angular_rigid_operation, create_direction, create_geographic_crs, create_map_conversion,
    create_map_conversion_scaled, create_projected_crs, create_representation_context,
    create_representation_subcontext, create_rigid_operation, create_well_known_text,
    GeographicCrsDraft, MapConversionDraft, ProjectedCrsDraft,
};
#[cfg(feature = "transform")]
pub use context::compose_project_frame;
pub use context::{coordinate_operation_for, resolve_operation_source, OperationSource};
pub use conversion::{
    resolve_geographic_offset_in, resolve_project_to_map, resolve_project_to_map_in,
    GeographicOffset, OperationKind, ProjectToMap,
};
pub use crs::{AngleUnit, GeographicCrs, LengthUnit, ProjectedCrs};
pub use error::{GeorefError, GeorefResult};
pub use north::{
    grid_north_direction, project_north_direction, resolve_true_north, NorthReference,
};
pub use site::{
    relate_site_elevation, site_reference, CompoundPlaneAngle, SiteElevationCheck,
    SiteElevationComparison, SiteReference, SITE_ELEVATION_TOLERANCE_M,
};
pub use view::GeorefView;

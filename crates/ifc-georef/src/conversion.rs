//! Coordinate operations: local engineering to map coordinates.

//! ## Internal split
//!
//! - `operation.rs`: the resolved value, the entry points and dispatch.
//! - `map.rs`: `IfcMapConversion` and `IfcMapConversionScaled`.
//! - `rigid.rs`: `IfcRigidOperation`, length and plane-angle forms.
//! - `pset.rs`: IFC2X3 `ePSet_MapConversion` and `ePSet_ProjectedCRS`.

mod map;
mod operation;
mod pset;
mod rigid;

pub use operation::{
    resolve_project_to_map, resolve_project_to_map_in, OperationKind, ProjectToMap,
};
pub use pset::{
    has_property_set_georeferencing, resolve_property_set_georeferencing, MAP_CONVERSION_PSET,
    PROJECTED_CRS_PSET,
};
pub use rigid::{resolve_geographic_offset_in, GeographicOffset};

//! Coordinate operations: local engineering to map coordinates.

//! ## Internal split
//!
//! - `operation.rs`: the resolved value, the entry points and dispatch.
//! - `map.rs`: `IfcMapConversion` and `IfcMapConversionScaled`.
//! - `rigid.rs`: `IfcRigidOperation`, length and plane-angle forms.

mod map;
mod operation;
mod rigid;

pub use operation::{
    resolve_project_to_map, resolve_project_to_map_in, OperationKind, ProjectToMap,
};
pub use rigid::{resolve_geographic_offset_in, GeographicOffset};

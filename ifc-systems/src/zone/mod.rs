//! `IfcZone` and spatial grouping.
//!
//! Zones are systems (`IfcZone -> IfcSystem`), so discovery lives in
//! `system`. What is here is zone-SPECIFIC: the WR1 member restriction, and
//! the spatial structure links that say where elements physically sit.
//!
//! ## Internal split
//!
//! - `definition.rs`: zones and WR1 member validity.
//! - `spatial_group.rs`: containment vs referencing.

mod definition;
mod spatial_group;

pub use definition::{zones, Zone};
pub use spatial_group::{spatial_placements, SpatialPlacement};

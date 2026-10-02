//! `IfcSystem`, `IfcDistributionSystem` and grouping.
//!
//! ## Internal split
//!
//! - `group.rs`: IfcSystem and group semantics.
//! - `services.rs`: what a system serves (`IfcRelServicesBuildings`, and
//!   the IFC4X3 `ServicesFacilities` references).

pub(crate) mod group;
pub(crate) mod services;

pub use group::{systems, System};

//! `IfcSystem`, `IfcDistributionSystem` and grouping.
//!
//! ## Internal split
//!
//! - `group.rs`: IfcSystem and group semantics.

pub(crate) mod group;

pub use group::{systems, System};

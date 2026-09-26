//! `IfcSystem`, `IfcDistributionSystem` and grouping.
//!
//! ## Internal split
//!
//! - `group.rs`: IfcSystem and group semantics.
//! - `distribution.rs`: distribution systems.

mod distribution;
pub(crate) mod group;

pub use group::{systems, System};

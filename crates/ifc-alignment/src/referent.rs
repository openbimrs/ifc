//! `IfcReferent` stationing and chainage.
//!
//! Stationing is not arc length: it restarts at equations and can run backwards.
//! Treating them as interchangeable is the classic linear-referencing bug.

//! ## Internal split
//!
//! - `station.rs`: one referent's `Pset_Stationing`, and the model-wide
//!   table.
//! - `stationing.rs`: one alignment's stationing, and station to distance
//!   along lookup.

mod station;
mod stationing;

#[allow(deprecated)]
pub use station::station_equations;
pub use station::StationEquation;
pub use stationing::{Stationing, STATION_TOLERANCE};

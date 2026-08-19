//! `IfcProjectedCRS` and geographic CRS identification.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `projected.rs`: IfcProjectedCRS.
//! - `identifier.rs`: authority/name/datum metadata.

mod identifier;
mod projected;

mod unit;

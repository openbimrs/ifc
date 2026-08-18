//! `ifc-properties` -- Property sets, quantities and units -- the non-geometric payload most
//!
//! consumers actually want.
//!
//! `references/ifc-spec/` ships the official property set definitions as XML:
//! 317 for IFC2x3 and 420 for IFC4. That is a machine-readable catalogue, so
//! standard Psets are data here rather than hand-written tables.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`pset`] | `IfcPropertySet` and single/enumerated/list/table properties |
//! | [`quantity`] | `IfcElementQuantity`: length, area, volume, weight, count |
//! | [`template`] | `IfcPropertySetTemplate` and property templates |
//! | [`standard`] | The official Pset catalogue from the shipped XML definitions |
//! | [`mod@unit`] | Unit assignment, prefixes and conversion-based units |
//! | [`value`] | `IfcValue` measure types and their interpretation |
//! | [`query`] | Lookup helpers: property by name, pset by element |
//! | [`error`] | Why a property lookup failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod error;
pub mod pset;
pub mod quantity;
pub mod query;
pub mod standard;
pub mod template;
pub mod unit;
pub mod value;

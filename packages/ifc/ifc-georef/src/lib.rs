//! `ifc-georef` -- Georeferencing: map conversion, coordinate reference systems, true north.
//!
//!
//! 8 entities in IFC4. Small in entity count, disproportionately important:
//! getting this wrong puts an entire model kilometres from where it belongs.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`crs`] | `IfcProjectedCRS` and geographic CRS identification |
//! | [`conversion`] | `IfcMapConversion`: local engineering to map coordinates |
//! | [`north`] | True north versus project north |
//! | [`elevation`] | Site elevation and height datums |
//! | [`error`] | Why a georeferencing operation failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod conversion;
pub mod crs;
pub mod elevation;
pub mod error;
pub mod north;

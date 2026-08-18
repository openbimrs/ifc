//! The 23 `IfcProfileDef` subtypes to 2D profiles.
//!
//! Every extrusion starts from a profile. IFC4 declares 23 profile entities:
//! parameterised ones (`IfcIShapeProfileDef`, `IfcRectangleProfileDef`, ...),
//! arbitrary ones backed by a polyline, and composites/derived ones that
//! transform another profile.
//!
//! Lowering targets `geom-profile`; this module only translates IFC parameters
//! into that crate's neutral types.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.

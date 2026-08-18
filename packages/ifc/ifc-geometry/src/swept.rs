//! Swept solids: extrusion, revolution, swept disk.
//!
//! Covers the 11 swept forms in IFC4, of which `IfcExtrudedAreaSolid` is by far
//! the most common in building models -- most walls, slabs and columns are one.
//!
//! Delegates the actual sweeping to `geom-sweep`.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.

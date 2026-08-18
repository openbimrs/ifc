//! Pre-tessellated face sets.
//!
//! `IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` arrive already discrete,
//! so this is the cheapest path -- mostly index rebasing (IFC indices are
//! 1-based) and optional normal/UV handling.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.

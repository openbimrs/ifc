//! Boundary representation: shells, faces, loops.
//!
//! `IfcFacetedBrep` (planar faces) and `IfcAdvancedBrep` (curved faces, IFC4+).
//! Handles the `IfcPolyLoop` / `IfcFaceOuterBound` structure and orientation
//! flags, which are a frequent source of inverted normals in real files.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.

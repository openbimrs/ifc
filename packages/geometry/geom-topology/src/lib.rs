//! `geom-topology` — exact boundary representation.
//!
//! # Why this is its own crate (and why it is the hard one)
//!
//! IFC4 carries ~37 topology entities. This is the crate that must earn the
//! "IfcOpenShell alternative" claim, because exact topology is precisely what
//! OpenCascade provides. It is deliberately separated from `geom-mesh`: a mesh
//! approximates a shape with triangles, a B-rep *is* the shape, with faces
//! lying on real surfaces and edges on real curves.
//!
//! # Scope
//!
//! - The topological hierarchy: vertex → edge → loop → face → shell → solid
//! - Half-edge (or equivalent) adjacency for traversal
//! - Orientation and manifoldness checking
//! - `IfcAdvancedBrep` (faces on analytic/NURBS surfaces) and faceted B-rep
//! - Euler characteristic validation — cheap, catches a large class of errors
//!
//! # Scope discipline
//!
//! **Target the surfaces real IFC files contain**: plane, cylinder, cone,
//! sphere, torus, plus extrusion and revolution, plus NURBS where authored.
//! That is tractable. A general NURBS-on-NURBS intersection engine is a
//! multi-year effort and is out of scope until evidence says otherwise.

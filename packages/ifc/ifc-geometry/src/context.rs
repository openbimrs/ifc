//! Representation context and precision.
//!
//! `IfcGeometricRepresentationContext` carries the model precision and the
//! true-north/world coordinate system. Sub-contexts distinguish 'Body' from
//! 'Axis', 'Box' and 'FootPrint' representations -- lowering the wrong one is a
//! common source of missing or duplicated geometry.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.

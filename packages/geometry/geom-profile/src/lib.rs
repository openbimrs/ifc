//! `geom-profile` — 2D cross-sections and polygon operations.
//!
//! # Why this is its own crate
//!
//! IFC4 defines **23 `IfcProfileDef` subtypes** — I/L/T/U/Z/C shapes, circles,
//! rectangles (hollow and rounded), ellipses, trapezia, plus arbitrary,
//! composite, derived and mirrored profiles. Nearly every solid in a real model
//! begins as one of these swept along something.
//!
//! All of it is strictly 2D, which makes it independently testable: a profile's
//! area and centroid can be checked in closed form against the standard's own
//! formulae, with no 3D machinery involved.
//!
//! # Scope
//!
//! - Parameterized shapes → boundary polygon (the 23 subtypes)
//! - Polygon predicates: orientation, self-intersection, containment
//! - 2D boolean (union/difference/intersection) for profiles with voids
//! - Triangulation of holed polygons
//!
//! # Not here
//!
//! Anything with a Z coordinate. Sweeping a profile is `geom-sweep`'s job.

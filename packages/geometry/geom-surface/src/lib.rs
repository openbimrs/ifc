//! `geom-surface` — parametric surface evaluation.
//!
//! # Why this is its own crate
//!
//! IFC4 has **37 surface entities**. B-rep is meaningless without them: a face
//! is a bounded region *of a surface*, so exact topology (`geom-topology`)
//! depends on being able to evaluate the underlying geometry.
//!
//! # Scope
//!
//! - Elementary: plane, cylinder, cone, sphere, torus
//! - Swept: surface of linear extrusion, surface of revolution
//! - B-spline / rational B-spline (NURBS) patches
//! - Curve-bounded and rectangular-trimmed surfaces
//! - Point/normal at (u,v), and projection of a point onto the surface
//!
//! # Why elementary surfaces come first
//!
//! Real building models are overwhelmingly planar, with cylinders for pipes and
//! columns. Elementary surfaces plus extrusion cover the vast majority of
//! geometry in practice; NURBS matters for the remainder and is implemented
//! second, not first.

//! `geom-curve` — parametric curve evaluation.
//!
//! # Why this is its own crate
//!
//! IFC4 carries **36 curve entities** and IFC4x3 adds transition spirals
//! (`IfcClothoid`, `IfcCosineSpiral`) for civil alignments. A curve is defined
//! by what it answers — point at parameter, tangent, arc length, closest
//! parameter to a point — and those questions are the same whether the curve is
//! a line, an arc, a B-spline or a clothoid.
//!
//! Keeping evaluation here means `geom-sweep` and `geom-surface` consume one
//! interface instead of matching on curve kinds.
//!
//! # Scope
//!
//! - Line, circle, ellipse, polyline, indexed poly-curve
//! - B-spline / rational B-spline (NURBS) curves with knots
//! - Composite curves and segment continuity
//! - Trimming (by parameter or by cartesian point) and offsets
//! - Arc-length parameterisation — needed for sweeps and for alignment
//!   stationing, and the most common source of subtle error
//!
//! # Scope discipline
//!
//! NURBS is where a geometry kernel balloons into a multi-year CAD project.
//! The target is **what real IFC files contain**: degree ≤ 3 in practice,
//! evaluation and tessellation. Curve/curve intersection and surface
//! interrogation are explicitly out until a fixture demands them.

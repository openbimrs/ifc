//! `ifc-alignment` — linear referencing and alignments (IFC4x3).
//!
//! # Why this is its own crate
//!
//! IFC4x3 adds **116 entities** over IFC4, and alignment is the core of them:
//! `IfcAlignment` with horizontal, vertical and cant layers, plus
//! `IfcLinearPlacement`, `IfcReferent`, `IfcPointByDistanceExpression`, and the
//! transition spirals (`IfcClothoid`, `IfcCosineSpiral`, `IfcSineSpiral`).
//!
//! This is infrastructure geometry — roads, rail, bridges. A consumer working
//! on buildings should not compile clothoid evaluation, which is why it is
//! separate rather than folded into `ifc-geometry`.
//!
//! # Scope
//!
//! - Horizontal alignment (lines, arcs, transition spirals)
//! - Vertical alignment (grades, parabolic/circular vertical curves)
//! - Cant (superelevation) for rail
//! - Linear placement: position an element by *station along an alignment*
//!   rather than by cartesian transform
//! - Station ↔ cartesian conversion in both directions
//!
//! # The hard part
//!
//! Linear placement requires **arc-length parameterisation** of the alignment
//! curve. For clothoids this has no closed form and needs numerical
//! integration, where accuracy directly becomes positional error on a
//! kilometre-scale object.

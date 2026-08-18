//! `geom-sweep` — profile + path → solid.
//!
//! # Why this is its own crate
//!
//! IFC4 defines **11 swept-solid forms** (extruded, revolved, tapered variants,
//! swept-disk, surface-curve-swept, sectioned spine), and IFC4x3 adds
//! directrix-derived variants. This is the single most common way real building
//! geometry is authored — walls, beams, columns, pipes and ducts are nearly all
//! sweeps.
//!
//! It sits above `geom-profile` (what is swept) and `geom-curve` (along what),
//! and below the boolean kernel (openings are cut afterwards).
//!
//! # Scope
//!
//! - Linear extrusion, with and without taper
//! - Revolution about an axis, including partial sweeps
//! - Sweep along an arbitrary directrix with a reference direction
//! - Swept disk (pipes) — circular profile along a curve
//! - Loft between sections (`IfcSectionedSpine`)
//!
//! # The hard part
//!
//! Self-intersection when the directrix curvature is tighter than the profile.
//! A pipe elbow whose bend radius is smaller than the pipe radius produces a
//! self-overlapping solid; detecting that is a documented failure, not a panic.

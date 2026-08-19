//! Supertype chain walking and subtype tests.
//!
//! # Why this is its own module
//!
//! `is_a("IfcWall", "IfcBuiltElement")` is the single most-called query in any
//! IFC tool — every filter, rule and selector runs it. It is also the query
//! most affected by schema drift: the answer differs between IFC4 and IFC4x3
//! because `IfcBuildingElement` was renamed `IfcBuiltElement`.
//!
//! Isolating it means the inevitable optimization (interning names, precomputed
//! ancestor bitsets) happens in one file, and the version-drift handling has an
//! obvious home.
//!
//! Not yet implemented -- Stage 1 in `../PLAN.md`.

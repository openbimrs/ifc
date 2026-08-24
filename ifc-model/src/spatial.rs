//! The containment tree: project, site, building, storey, space.
//!
//! Built from `IfcRelAggregates` and `IfcRelContainedInSpatialStructure`.
//!
//! # Pitfall
//!
//! Real files omit levels, duplicate storeys, or attach elements directly to the
//! building. The tree must tolerate that rather than assume the canonical shape.
//!
//! Not yet implemented -- see `../PLAN.md`.

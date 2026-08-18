//! `ifc-structural` — the structural analysis model.
//!
//! # Why this is its own crate
//!
//! IFC4 defines **39 structural entities**, and they describe an *analysis*
//! model that is deliberately distinct from the physical one: a beam is a
//! curve member with a section and end releases, not a solid. Two different
//! idealisations of the same building coexist in one file.
//!
//! # Scope
//!
//! - Analysis model, curve/surface/point members and connections
//! - Actions (loads) and reactions; linear, planar, and point variants
//! - Load cases, load groups, load combinations
//! - Boundary conditions and connection stiffness
//!
//! # Why an IFC library should carry this at all
//!
//! Structural analysis is one of the few areas where IFC is genuinely used for
//! round-trip exchange rather than one-way handover, so it is a real
//! interoperability target rather than a checkbox.

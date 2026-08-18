//! `ifc-model` -- Indexed semantic views over a parsed model: type buckets, the spatial
//!
//! containment tree, relationship indices and attribute access.
//!
//! This crate holds **no geometry**. An element's shape is produced by
//! `ifc-geometry`; here an element is identity, attributes and relationships.
//! Keeping them apart is what lets a property or quantity consumer skip the
//! geometry stack entirely.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`entity`] | One entity instance: id, type, attribute slots |
//! | [`model`] | The `Model` container and construction from parsed records |
//! | [`index`] | Type buckets and reverse-reference indices |
//! | [`spatial`] | The containment tree: project, site, building, storey, space |
//! | [`relation`] | Objectified relationships (`IfcRel*`) |
//! | [`traverse`] | Graph walks with cycle protection |
//! | [`guid`] | IFC GlobalId encoding and decoding |
//! | [`error`] | Why a model query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod entity;
pub mod error;
pub mod guid;
pub mod index;
pub mod model;
pub mod relation;
pub mod spatial;
pub mod traverse;

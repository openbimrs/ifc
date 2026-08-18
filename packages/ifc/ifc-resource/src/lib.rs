//! `ifc-resource` -- Construction resources: labour, equipment, material and crew.
//!
//!
//! 21 entities in IFC4. Resources link the schedule to cost -- a task consumes
//! a resource, and the resource carries a rate.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`resource`] | The resource hierarchy and common attributes |
//! | [`labour`] | `IfcLaborResource` and crew composition |
//! | [`equipment`] | `IfcConstructionEquipmentResource` |
//! | [`material`] | `IfcConstructionMaterialResource` |
//! | [`usage`] | Resource time, quantity and levelling |
//! | [`error`] | Why a resource query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod equipment;
pub mod error;
pub mod labour;
pub mod material;
pub mod resource;
pub mod usage;

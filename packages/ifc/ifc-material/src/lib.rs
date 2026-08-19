//! `ifc-material` -- Material definitions: layer sets, profile sets, constituents and usage.
//!
//!
//! 22 entities in IFC4. Material layer sets are what let a wall know it is
//! 200 mm concrete plus 60 mm insulation, which is required for quantity
//! takeoff and thermal analysis alike.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `material` | `IfcMaterial` and material properties |
//! | `layer` | `IfcMaterialLayerSet` and `IfcMaterialLayerSetUsage` |
//! | `profile` | `IfcMaterialProfileSet` for profiled members |
//! | `constituent` | `IfcMaterialConstituentSet` for non-layered composites |
//! | `usage` | Resolving which material applies to a given element |
//! | `error` | Why a material lookup failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `PLAN.md` for the stage that fills them.

mod constituent;
mod error;
mod layer;
mod material;
mod profile;
mod usage;

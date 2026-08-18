//! `ifc-cost` -- Cost schedules, cost items and cost values -- the 5D layer.
//!
//!
//! Separate from `ifc-schedule` because plenty of consumers want cost without
//! time, or time without cost.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`item`] | `IfcCostItem` and the cost breakdown tree |
//! | [`schedule`] | `IfcCostSchedule` and its predefined types |
//! | [`value`] | `IfcCostValue`, applied values and rates |
//! | [`takeoff`] | Binding cost items to quantities |
//! | [`error`] | Why a cost query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod error;
pub mod item;
pub mod schedule;
pub mod takeoff;
pub mod value;

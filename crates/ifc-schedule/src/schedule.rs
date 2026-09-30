//! Work plans and work schedules.
//!
//! ## Internal split
//!
//! - `work_control.rs`: `IfcWorkPlan` and `IfcWorkSchedule`, which share every
//!   slot through `IfcWorkControl`.

mod work_control;

pub use work_control::{
    slot as work_control_slot, work_plans, work_schedules, AuthoredDateTime, AuthoredDuration,
    WorkControl, WorkControlKind,
};

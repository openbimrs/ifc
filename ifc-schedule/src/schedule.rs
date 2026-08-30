//! Work plans and work schedules.
//!
//! ## Internal split
//!
//! - `work_control.rs`: `IfcWorkPlan` and `IfcWorkSchedule`, which share every
//!   slot through `IfcWorkControl`.
//! - `plan.rs`, `work_schedule.rs`: planned owners, kept until either type
//!   needs behaviour the other does not share.

mod plan;
mod work_control;
mod work_schedule;

pub use work_control::{work_plans, work_schedules, WorkControl, WorkControlKind};

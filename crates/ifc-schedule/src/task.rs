//! Tasks and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcTask` and `IfcTaskTime` (with its
//!   `IfcTaskTimeRecurring` subtype), including the milestone
//!   contradiction `IfcTaskTime` `WR1` describes.
//! - `schedule_time_control.rs`: IFC2X3 `IfcScheduleTimeControl`, reached
//!   through `IfcRelAssignsTasks.TimeForTask`.

pub(crate) mod definition;
mod schedule_time_control;

pub use definition::{tasks, DurationType, Task, TaskTime, TaskTimeAnomaly};
pub use schedule_time_control::ScheduleTimeControl;

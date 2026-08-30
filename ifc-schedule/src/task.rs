//! Tasks and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcTask` and `IfcTaskTime`, including the milestone
//!   contradiction `IfcTaskTime` `WR1` describes.
//! - `time.rs`: planned owner for time variants beyond `IfcTaskTime`.

mod definition;
mod time;

pub use definition::{tasks, DurationType, Task, TaskTime, TaskTimeAnomaly};

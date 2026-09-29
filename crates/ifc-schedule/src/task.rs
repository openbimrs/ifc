//! Tasks and their stated times.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcTask` and `IfcTaskTime`, including the milestone
//!   contradiction `IfcTaskTime` `WR1` describes.

pub(crate) mod definition;

pub use definition::{tasks, DurationType, Task, TaskTime, TaskTimeAnomaly};

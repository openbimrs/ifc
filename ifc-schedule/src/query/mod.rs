//! Bounded, deterministic schedule queries.
//!
//! ## Internal split
//!
//! - `timeline.rs`: membership, start/end tasks, and execution ordering.

mod timeline;

pub use timeline::{end_tasks, execution_order, start_tasks, subtasks_of, tasks_of_schedule};

//! Bounded, deterministic schedule queries.
//!
//! ## Internal split
//!
//! - `timeline.rs`: membership, start/end tasks, and execution ordering.

mod timeline;

pub use timeline::{
    assigns as assigns_slot, end_tasks, execution_order, nests as nests_slot, start_tasks,
    subtasks_of, tasks_of_schedule,
};

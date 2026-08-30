//! `ifc-schedule` — construction scheduling as a **view** over the model.
//!
//! # This crate owns no data and starts no work
//!
//! It borrows a `&Model` and interprets the entities that happen to be
//! scheduling entities. It does not run jobs, touch the wall clock, or decide
//! what "now" is: a schedule in a file is authored intent, and this crate
//! reports that intent exactly as stated.
//!
//! # What is read, and what is deliberately not computed
//!
//! Read: plans, schedules, tasks, task times, sequences with lag, calendars
//! with recurrence patterns, events, and the orderings those imply.
//!
//! Not computed: dates, durations in real time, or the critical path. Every
//! date in IFC is an ISO 8601 string and every duration an ISO 8601 duration;
//! turning those into a timeline needs a date library and calendar expansion.
//! This crate's only dependency is `ifc-model`, so it returns the authored
//! strings intact and leaves arithmetic to a caller who already has a date
//! library and knows which calendar applies.
//!
//! What it does supply is the part that arithmetic needs and cannot recover on
//! its own: the sequence graph, its cycles, and a deterministic execution
//! order.
//!
//! # Modules
//!
//! | Module | Role |
//! | --- | --- |
//! | [`schedule`] | `IfcWorkPlan` and `IfcWorkSchedule` |
//! | [`task`] | `IfcTask` and `IfcTaskTime` |
//! | [`sequence`] | `IfcRelSequence`, lag, and cycle reporting |
//! | [`calendar`] | `IfcWorkCalendar` and recurrence patterns |
//! | [`event`] | `IfcEvent` and `IfcEventTime` |
//! | [`query`] | Deterministic membership and ordering queries |
//! | [`error`] | Contradictions a file can state |

pub mod calendar;
pub mod error;
pub mod event;
pub mod query;
mod recurrence;
pub mod schedule;
pub mod sequence;

pub use calendar::{
    work_calendars, Recurrence, RecurrenceType, WorkCalendar, WorkTime, WorkTimeRole,
};
pub use error::{SequenceCycle, TaskTimeAnomaly};
pub use event::{events, Event, EventTime};
pub use query::{end_tasks, execution_order, start_tasks, subtasks_of, tasks_of_schedule};
pub use schedule::{work_plans, work_schedules, WorkControl, WorkControlKind};
pub use sequence::{
    downstream_of, find_cycle, predecessors_of, sequences, successors_of, Lag, Sequence,
    SequenceType, MAX_SEQUENCE_DEPTH,
};
pub use task::{tasks, DurationType, Task, TaskTime};

pub mod task;

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
//! This crate depends only on `ifc-model` and the `ifc-schema` release
//! tables, so it returns the authored strings intact and leaves arithmetic to a caller who already has a date
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

pub mod authoring;
pub mod calendar;
pub mod error;
pub mod event;
pub mod query;
mod recurrence;
mod release;
pub mod schedule;
pub mod sequence;

pub use authoring::{
    assign_tasks_to_control, assign_tasks_to_control_with_owner_history, create_event,
    create_event_time, create_event_with_owner_history, create_lag_time, create_lag_time_in,
    create_procedure, create_procedure_with_owner_history, create_recurrence_pattern,
    create_sequence, create_sequence_with_owner_history, create_task, create_task_time,
    create_task_time_recurring, create_task_with_owner_history, create_time_period,
    create_work_calendar, create_work_calendar_with_owner_history, create_work_control,
    create_work_control_with_owner_history, create_work_time, nest_tasks,
    nest_tasks_with_owner_history, CalendarDate, DateTimeValue, EventDraft, EventTimeDraft,
    LocalTime, ProcedureDraft, RecurrenceDraft, ScheduleAuthoringResult, TaskDraft, TaskTimeDraft,
    TimeLag, WorkControlDraft,
};
pub use calendar::{
    work_calendars, Recurrence, RecurrenceType, WorkCalendar, WorkTime, WorkTimeRole,
};
pub use error::{ScheduleReadError, SequenceCycle, TaskTimeAnomaly};
pub use event::{events, Event, EventTime};
/// The IFC release a schedule record is written against (re-exported from
/// `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use query::{end_tasks, execution_order, start_tasks, subtasks_of, tasks_of_schedule};
pub use schedule::{
    work_plans, work_schedules, AuthoredDateTime, AuthoredDuration, WorkControl, WorkControlKind,
};
pub use sequence::{
    downstream_of, find_cycle, predecessors_of, sequences, successors_of, Lag, Sequence,
    SequenceType, MAX_SEQUENCE_DEPTH,
};
pub use task::{tasks, DurationType, Task, TaskTime};

pub mod task;

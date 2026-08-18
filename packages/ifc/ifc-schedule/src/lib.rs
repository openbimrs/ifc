//! `ifc-schedule` -- Work schedules, tasks, sequencing and calendars -- the 4D layer.
//!
//!
//! 19 process entities in IFC4, including the event and lag-time machinery.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`task`] | `IfcTask`, task time and predefined types |
//! | [`schedule`] | `IfcWorkSchedule`, `IfcWorkPlan`, `IfcWorkCalendar` |
//! | [`sequence`] | `IfcRelSequence`: predecessors, successors and lag |
//! | [`calendar`] | Working times, exceptions and recurrence |
//! | [`event`] | `IfcEvent` and event triggers |
//! | [`error`] | Why a schedule query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod calendar;
pub mod error;
pub mod event;
pub mod schedule;
pub mod sequence;
pub mod task;

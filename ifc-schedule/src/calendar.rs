//! Work calendars, working periods, and recurrence.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcWorkCalendar`, `IfcWorkTime` and
//!   `IfcRecurrencePattern`.
//! - `working_time.rs`: planned owner for expanded working periods.

mod definition;
mod working_time;

pub use definition::{
    work_calendars, Recurrence, RecurrenceType, WorkCalendar, WorkTime, WorkTimeRole,
};

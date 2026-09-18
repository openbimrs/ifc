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
    recurrence_slot, slot as work_calendar_slot, work_calendars, work_time_slot, Recurrence,
    RecurrenceType, WorkCalendar, WorkTime, WorkTimeRole,
};

//! Work calendars, working periods, and recurrence.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcWorkCalendar`, `IfcWorkTime` and
//!   `IfcRecurrencePattern`.

mod definition;

pub use definition::{
    recurrence_slot, slot as work_calendar_slot, work_calendars, work_time_slot, Recurrence,
    RecurrenceType, WorkCalendar, WorkTime, WorkTimeRole,
};

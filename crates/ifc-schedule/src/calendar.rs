//! Work calendars, working periods, and recurrence.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcWorkCalendar`, `IfcWorkTime`,
//!   `IfcRecurrencePattern` and `IfcTimePeriod`.

mod definition;

pub use definition::{
    recurrence_pattern, recurrence_slot, slot as work_calendar_slot, time_period_slot,
    work_calendars, work_time_slot, Recurrence, RecurrenceType, TimePeriod, WorkCalendar, WorkTime,
    WorkTimeRole,
};

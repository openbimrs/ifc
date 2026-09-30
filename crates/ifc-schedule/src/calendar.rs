//! Work calendars, working periods, and recurrence.
//!
//! ## Internal split
//!
//! - `definition.rs`: `IfcWorkCalendar`, `IfcWorkTime`,
//!   `IfcRecurrencePattern` and `IfcTimePeriod`.

pub(crate) mod definition;

#[allow(deprecated)]
pub use definition::work_calendars;
pub use definition::{
    read_work_calendars, recurrence_pattern, recurrence_slot, slot as work_calendar_slot,
    time_period_slot, work_time_slot, Recurrence, RecurrenceType, TimePeriod, WorkCalendar,
    WorkTime, WorkTimeRole,
};

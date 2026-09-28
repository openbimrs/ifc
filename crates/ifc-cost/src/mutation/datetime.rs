//! Date and time values, in the form each release declares (#214).
//!
//! IFC4 and IFC4X3 declare `IfcCostSchedule.SubmittedOn` and `UpdateDate` as
//! `IfcDateTime`, an ISO 8601 string. IFC2X3 TC1 declares them
//! `IfcDateTimeSelect`, a SELECT of three entities:
//!
//! ```text
//! TYPE IfcDateTimeSelect = SELECT (IfcCalendarDate, IfcLocalTime, IfcDateAndTime);
//! ENTITY IfcCalendarDate;  DayComponent, MonthComponent, YearComponent
//! ENTITY IfcLocalTime;     HourComponent, MinuteComponent (OPTIONAL),
//!                          SecondComponent (OPTIONAL), Zone (OPTIONAL),
//!                          DaylightSavingOffset (OPTIONAL)
//! ENTITY IfcDateAndTime;   DateComponent, TimeComponent
//! ```
//!
//! [`DateTimeValue`] carries either form. The text form is written as given;
//! a record form is checked against the schema's own rules
//! (`IfcValidCalendarDate`, `IfcLeapYear`, `IfcValidTime` and the ranges of
//! `IfcMonthInYearNumber`, `IfcHourInDay`, `IfcMinuteInHour` and
//! `IfcSecondInMinute`) and staged as those records by the writer that
//! references it. `ifc-schedule` carries an identical copy of these types:
//! sibling domain crates may not depend on each other, and only `ifc-model`
//! and `ifc-schema` sit below both.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::CostAuthoringError;

/// A date, a time or both, as one release or another declares it.
///
/// Converts from `&str` for the IFC4/IFC4X3 text form, and from
/// [`CalendarDate`] and [`LocalTime`] for the IFC2X3 record forms.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DateTimeValue<'a> {
    /// IFC4 and IFC4X3 `IfcDateTime`: ISO 8601 text, written as given.
    Text(&'a str),
    /// IFC2X3 `IfcCalendarDate`.
    Date(CalendarDate),
    /// IFC2X3 `IfcLocalTime`.
    Time(LocalTime),
    /// IFC2X3 `IfcDateAndTime`: an `IfcCalendarDate` and an `IfcLocalTime`.
    DateAndTime(CalendarDate, LocalTime),
}

impl Default for DateTimeValue<'_> {
    fn default() -> Self {
        Self::Text("")
    }
}

impl<'a> From<&'a str> for DateTimeValue<'a> {
    fn from(text: &'a str) -> Self {
        Self::Text(text)
    }
}

impl From<CalendarDate> for DateTimeValue<'_> {
    fn from(date: CalendarDate) -> Self {
        Self::Date(date)
    }
}

impl From<LocalTime> for DateTimeValue<'_> {
    fn from(time: LocalTime) -> Self {
        Self::Time(time)
    }
}

/// An IFC2X3 `IfcCalendarDate`: a Gregorian day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CalendarDate {
    /// `YearComponent`.
    pub year: i64,
    /// `MonthComponent`, 1 = January through 12 = December.
    pub month: i64,
    /// `DayComponent`, a day that exists in that month.
    pub day: i64,
}

impl CalendarDate {
    /// A date; checked when a writer stages it.
    #[must_use]
    pub const fn new(year: i64, month: i64, day: i64) -> Self {
        Self { year, month, day }
    }
}

/// An IFC2X3 `IfcLocalTime`: a time of day.
///
/// `Zone` and `DaylightSavingOffset` are not carried yet and are written
/// `$`, which the schema allows.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LocalTime {
    /// `HourComponent`, 0 through 23.
    pub hour: i64,
    /// `MinuteComponent`, 0 through 59.
    pub minute: Option<i64>,
    /// `SecondComponent`, at least 0 and below 60. Requires a minute
    /// (`IfcValidTime`).
    pub second: Option<f64>,
}

impl LocalTime {
    /// A whole hour; add a minute and second with the setters.
    #[must_use]
    pub const fn new(hour: i64) -> Self {
        Self {
            hour,
            minute: None,
            second: None,
        }
    }

    /// Sets `MinuteComponent`.
    #[must_use]
    pub const fn minute(mut self, value: i64) -> Self {
        self.minute = Some(value);
        self
    }

    /// Sets `SecondComponent`.
    #[must_use]
    pub const fn second(mut self, value: f64) -> Self {
        self.second = Some(value);
        self
    }
}

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    expected: &'static str,
) -> CostAuthoringError {
    CostAuthoringError::InvalidValue {
        entity,
        attribute,
        reason: format!("expected {expected}"),
    }
}

fn leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

impl CalendarDate {
    fn check(self) -> Result<(), CostAuthoringError> {
        const ENTITY: &str = "IFCCALENDARDATE";
        if !(1..=12).contains(&self.month) {
            return Err(invalid(ENTITY, "MonthComponent", "a month in 1..=12"));
        }
        let last = match self.month {
            4 | 6 | 9 | 11 => 30,
            2 if leap_year(self.year) => 29,
            2 => 28,
            _ => 31,
        };
        if !(1..=last).contains(&self.day) {
            return Err(invalid(
                ENTITY,
                "DayComponent",
                "a day that exists in the month (IfcValidCalendarDate)",
            ));
        }
        Ok(())
    }

    fn stage(self, tx: &mut Transaction) -> EntityId {
        tx.create(Entity::new(
            "IFCCALENDARDATE",
            vec![
                Value::Integer(self.day),
                Value::Integer(self.month),
                Value::Integer(self.year),
            ],
        ))
    }
}

impl LocalTime {
    fn check(self) -> Result<(), CostAuthoringError> {
        const ENTITY: &str = "IFCLOCALTIME";
        if !(0..24).contains(&self.hour) {
            return Err(invalid(ENTITY, "HourComponent", "an hour in 0..24"));
        }
        if self.minute.is_some_and(|m| !(0..=59).contains(&m)) {
            return Err(invalid(ENTITY, "MinuteComponent", "a minute in 0..=59"));
        }
        if let Some(second) = self.second {
            if !second.is_finite() || !(0.0..60.0).contains(&second) {
                return Err(invalid(
                    ENTITY,
                    "SecondComponent",
                    "a finite second in 0.0..60.0",
                ));
            }
            if self.minute.is_none() {
                return Err(invalid(
                    ENTITY,
                    "MinuteComponent",
                    "a minute when a second is given (IfcValidTime)",
                ));
            }
        }
        Ok(())
    }

    fn stage(self, tx: &mut Transaction) -> EntityId {
        tx.create(Entity::new(
            "IFCLOCALTIME",
            vec![
                Value::Integer(self.hour),
                self.minute.map_or(Value::Null, Value::Integer),
                self.second.map_or(Value::Null, Value::Real),
                Value::Null,
                Value::Null,
            ],
        ))
    }
}

impl<'a> DateTimeValue<'a> {
    /// Check a record form against the schema's rules; text is checked by
    /// the writer that owns it.
    pub(super) fn check(self) -> Result<(), CostAuthoringError> {
        match self {
            Self::Text(_) => Ok(()),
            Self::Date(date) => date.check(),
            Self::Time(time) => time.check(),
            Self::DateAndTime(date, time) => {
                date.check()?;
                time.check()
            }
        }
    }

    /// The entity a record form is staged as; `None` for text.
    pub(super) const fn record_type(self) -> Option<&'static str> {
        match self {
            Self::Text(_) => None,
            Self::Date(_) => Some("IFCCALENDARDATE"),
            Self::Time(_) => Some("IFCLOCALTIME"),
            Self::DateAndTime(..) => Some("IFCDATEANDTIME"),
        }
    }

    /// The value to lay the owning record out with before anything is
    /// staged: the text, or `placeholder` standing in for the record.
    pub(super) fn provisional(self, placeholder: EntityId) -> Value {
        match self {
            Self::Text(text) => Value::Text(text.into()),
            _ => Value::Ref(placeholder),
        }
    }

    /// Stage the records of a record form and return the reference to
    /// write; `None` for text, which needs no record.
    pub(super) fn stage(self, tx: &mut Transaction) -> Option<EntityId> {
        match self {
            Self::Text(_) => None,
            Self::Date(date) => Some(date.stage(tx)),
            Self::Time(time) => Some(time.stage(tx)),
            Self::DateAndTime(date, time) => {
                let date = date.stage(tx);
                let time = time.stage(tx);
                Some(tx.create(Entity::new(
                    "IFCDATEANDTIME",
                    vec![Value::Ref(date), Value::Ref(time)],
                )))
            }
        }
    }
}

/// Placeholders for date references in a record laid out before the dates
/// are staged. Never staged: each is replaced by [`patch`] first.
pub(super) const PLACEHOLDERS: [EntityId; 3] = [
    EntityId(u64::MAX),
    EntityId(u64::MAX - 1),
    EntityId(u64::MAX - 2),
];

/// Stage each date's records and replace its placeholder in `record`.
pub(super) fn patch(
    tx: &mut Transaction,
    record: &mut Entity,
    dates: &[Option<DateTimeValue<'_>>],
) {
    for (date, placeholder) in dates.iter().zip(PLACEHOLDERS) {
        let Some(staged) = date.and_then(|date| date.stage(tx)) else {
            continue;
        };
        for slot in &mut record.attributes {
            if *slot == Value::Ref(placeholder) {
                *slot = Value::Ref(staged);
            }
        }
    }
}

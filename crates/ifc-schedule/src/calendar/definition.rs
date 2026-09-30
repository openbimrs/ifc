//! `IfcWorkCalendar`, the working/exception times it declares, and their
//! recurrence patterns and time periods.
//!
//! # Read by name in the declared release (#234)
//!
//! The readers bind the model's declared release like the task readers
//! (#212) and find every attribute by name in its table. IFC4 ADD2 TC1 and
//! IFC4X3 ADD2 lay the records out alike, except that IFC4X3 names
//! `IfcWorkTime.Start` and `Finish` `StartDate` and `FinishDate`. IFC2X3 TC1
//! declares none of these entities, so an IFC2X3 model has no calendars.
//! IFC4X1, IFC4X2 and a header with several schemas are refused.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! The slot modules below are the IFC4/IFC4X3 positions the modelless
//! writers lay records out with; the readers never go through them.
//!
//! `IfcWorkCalendar` is an `IfcControl`, so the first six slots are inherited:
//!
//! ```text
//! 0 GlobalId        1 OwnerHistory    2 Name
//! 3 Description     4 ObjectType      5 Identification   (IfcControl)
//! 6 WorkingTimes    7 ExceptionTimes  8 PredefinedType
//!
//! IfcWorkTime  (IfcSchedulingTime)
//! 0 Name           1 DataOrigin      2 UserDefinedDataOrigin
//! 3 RecurrencePattern    4 Start      5 Finish
//!
//! IfcRecurrencePattern
//! 0 RecurrenceType  1 DayComponent    2 WeekdayComponent
//! 3 MonthComponent  4 Position        5 Interval
//! 6 Occurrences     7 TimePeriods
//!
//! IfcTimePeriod
//! 0 StartTime       1 EndTime
//! ```
//!
//! # Working times and exception times are both `IfcWorkTime`
//!
//! They differ only by which slot holds them: slot 6 declares when work
//! happens, slot 7 declares when it does not. Same entity type, opposite
//! meaning -- so a reader that collects "all the IfcWorkTimes" and treats them
//! uniformly turns holidays into working days.

use ifc_model::{Entity, EntityId, Model, Value};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::SchemaVersion;

/// `IfcWorkCalendar` slots.
pub mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Identification` (from `IfcControl`).
    pub const IDENTIFICATION: usize = 5;
    /// `WorkingTimes`: when work happens.
    pub const WORKING_TIMES: usize = 6;
    /// `ExceptionTimes`: when it does not.
    pub const EXCEPTION_TIMES: usize = 7;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 8;
}

/// `IfcWorkTime` slots.
pub mod work_time_slot {
    /// `Name`.
    pub const NAME: usize = 0;
    /// `RecurrencePattern`.
    pub const RECURRENCE_PATTERN: usize = 3;
    /// `Start`.
    pub const START: usize = 4;
    /// `Finish`.
    pub const FINISH: usize = 5;
}

/// `IfcRecurrencePattern` slots.
pub mod recurrence_slot {
    /// `RecurrenceType`. Required by the schema.
    pub const RECURRENCE_TYPE: usize = 0;
    /// `DayComponent`.
    pub const DAY_COMPONENT: usize = 1;
    /// `WeekdayComponent`.
    pub const WEEKDAY_COMPONENT: usize = 2;
    /// `MonthComponent`.
    pub const MONTH_COMPONENT: usize = 3;
    /// `Position`, for `MONTHLY_BY_POSITION` and friends.
    pub const POSITION: usize = 4;
    /// `Interval`.
    pub const INTERVAL: usize = 5;
    /// `Occurrences`.
    pub const OCCURRENCES: usize = 6;
    /// `TimePeriods`, a list of `IfcTimePeriod` (#233).
    pub const TIME_PERIODS: usize = 7;
}

/// `IfcTimePeriod` slots (IFC4 and IFC4X3; IFC2X3 has no `IfcTimePeriod`).
pub mod time_period_slot {
    /// `StartTime`, an `IfcTime`. Required by the schema.
    pub const START_TIME: usize = 0;
    /// `EndTime`, an `IfcTime`. Required by the schema.
    pub const END_TIME: usize = 1;
}

/// Whether a period declares work or an exception to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkTimeRole {
    /// From `WorkingTimes`: work happens in this period.
    Working,
    /// From `ExceptionTimes`: work does not happen in this period.
    Exception,
}

/// How a work period repeats.
///
/// `IfcRecurrenceTypeEnum`, verified against IFC4 EXPRESS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecurrenceType {
    /// `.DAILY.`
    Daily,
    /// `.WEEKLY.`
    Weekly,
    /// `.MONTHLY_BY_DAY_OF_MONTH.`
    MonthlyByDayOfMonth,
    /// `.MONTHLY_BY_POSITION.`
    MonthlyByPosition,
    /// `.BY_DAY_COUNT.`
    ByDayCount,
    /// `.BY_WEEKDAY_COUNT.`
    ByWeekdayCount,
    /// `.YEARLY_BY_DAY_OF_MONTH.`
    YearlyByDayOfMonth,
    /// `.YEARLY_BY_POSITION.`
    YearlyByPosition,
}

impl RecurrenceType {
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "DAILY" => Self::Daily,
            "WEEKLY" => Self::Weekly,
            "MONTHLY_BY_DAY_OF_MONTH" => Self::MonthlyByDayOfMonth,
            "MONTHLY_BY_POSITION" => Self::MonthlyByPosition,
            "BY_DAY_COUNT" => Self::ByDayCount,
            "BY_WEEKDAY_COUNT" => Self::ByWeekdayCount,
            "YEARLY_BY_DAY_OF_MONTH" => Self::YearlyByDayOfMonth,
            "YEARLY_BY_POSITION" => Self::YearlyByPosition,
            _ => return None,
        })
    }
}

/// A recurrence pattern as authored.
///
/// Not expanded into concrete dates: expansion needs a calendar library and a
/// bounded window, and an unbounded pattern (`Occurrences` absent) has no
/// finite expansion at all. The stated shape is returned and expansion is left
/// to a caller who can supply both.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Recurrence {
    /// The entity.
    pub id: EntityId,
    /// How it repeats.
    pub recurrence_type: Option<RecurrenceType>,
    /// `DayComponent`: days of the month it applies to, 1 through 31
    /// (#233).
    pub days: Vec<i64>,
    /// Weekdays it applies to, 1 = Monday through 7 = Sunday.
    pub weekdays: Vec<i64>,
    /// `MonthComponent`: months it applies to, 1 = January through
    /// 12 = December (#233).
    pub months: Vec<i64>,
    /// The ordinal position within the period, for positional patterns.
    ///
    /// `MONTHLY_BY_POSITION` uses it as "the 2nd Tuesday"; a negative value
    /// counts from the end of the period.
    pub position: Option<i64>,
    /// The interval between occurrences.
    pub interval: Option<i64>,
    /// How many times it repeats, if bounded.
    pub occurrences: Option<i64>,
    /// `TimePeriods`: the times of day each occurrence spans, in authored
    /// order (#233). A reference that is not an `IfcTimePeriod` is not
    /// listed.
    pub time_periods: Vec<TimePeriod>,
}

/// An `IfcTimePeriod`: a start and end time of day, as authored (#233).
///
/// Both are `IfcTime` strings and are not parsed, for the same reason
/// dates are not: interpreting them is calendar arithmetic this crate
/// leaves to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TimePeriod {
    /// The `IfcTimePeriod` entity.
    pub id: EntityId,
    /// `StartTime`, as authored.
    pub start_time: Option<String>,
    /// `EndTime`, as authored.
    pub end_time: Option<String>,
}

impl Recurrence {
    /// Whether the pattern states a finite number of occurrences.
    ///
    /// An unbounded pattern is legal and common ("every Monday, forever"), so
    /// a caller expanding one must impose its own window.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.occurrences.is_some()
    }
}

/// One working or exception period.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct WorkTime {
    /// The `IfcWorkTime` entity.
    pub id: EntityId,
    /// Whether this declares work or an exception.
    pub role: WorkTimeRole,
    /// The period name, as authored.
    pub name: Option<String>,
    /// `DataOrigin` (`IfcDataOriginEnum`), the token without its dots
    /// (#234).
    pub data_origin: Option<String>,
    /// `UserDefinedDataOrigin`, as authored (#234).
    pub user_defined_data_origin: Option<String>,
    /// Start date, as authored: IFC4's `Start`, IFC4X3's `StartDate`.
    pub start: Option<String>,
    /// Finish date, as authored: IFC4's `Finish`, IFC4X3's `FinishDate`.
    pub finish: Option<String>,
    /// The recurrence pattern, if the period repeats.
    pub recurrence: Option<Recurrence>,
}

const CALENDAR: &str = "IFCWORKCALENDAR";
const WORK_TIME: &str = "IFCWORKTIME";
const PATTERN: &str = "IFCRECURRENCEPATTERN";
const PERIOD: &str = "IFCTIMEPERIOD";

/// A borrowed view of an `IfcWorkCalendar`, read against one release.
#[derive(Debug, Clone, Copy)]
pub struct WorkCalendar<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: ReadRelease,
}

impl<'m> WorkCalendar<'m> {
    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The release this calendar is read against.
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn text(&self, attribute: &'static str) -> Option<&'m str> {
        self.release.text(CALENDAR, self.entity, attribute)
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.text("GlobalId")
    }

    /// The calendar name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.text("Identification")
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        self.release.token(CALENDAR, self.entity, "PredefinedType")
    }

    /// Periods when work happens.
    #[must_use]
    pub fn working_times(&self, model: &Model) -> Vec<WorkTime> {
        self.times(model, "WorkingTimes", WorkTimeRole::Working)
    }

    /// Periods when work does not happen, such as holidays.
    #[must_use]
    pub fn exception_times(&self, model: &Model) -> Vec<WorkTime> {
        self.times(model, "ExceptionTimes", WorkTimeRole::Exception)
    }

    fn times(&self, model: &Model, attribute: &'static str, role: WorkTimeRole) -> Vec<WorkTime> {
        let mut refs = Vec::new();
        if let Some(v) = self.release.value(CALENDAR, self.entity, attribute) {
            v.for_each_ref(&mut |id| refs.push(id));
        }
        refs.into_iter()
            .filter_map(|id| read_work_time(self.release, model, id, role))
            .collect()
    }
}

fn read_work_time(
    release: ReadRelease,
    model: &Model,
    id: EntityId,
    role: WorkTimeRole,
) -> Option<WorkTime> {
    let entity = model.get(id)?;
    if !release.is_a(&entity.type_name, WORK_TIME) {
        return None;
    }
    let text = |attribute| {
        release
            .text(WORK_TIME, entity, attribute)
            .map(str::to_string)
    };
    let recurrence = release
        .reference(WORK_TIME, entity, "RecurrencePattern")
        .and_then(|pattern| read_recurrence(release, model, pattern));
    Some(WorkTime {
        id,
        role,
        name: text("Name"),
        data_origin: release
            .token(WORK_TIME, entity, "DataOrigin")
            .map(str::to_string),
        user_defined_data_origin: text("UserDefinedDataOrigin"),
        start: text("Start"),
        finish: text("Finish"),
        recurrence,
    })
}

/// The `IfcRecurrencePattern` `id`, read by name in the model's declared
/// release (#233, #234).
///
/// For patterns referenced from somewhere other than a work calendar, such
/// as `IfcTaskTimeRecurring.Recurrence` (also reachable through
/// [`TaskTime::recurrence`](crate::TaskTime::recurrence)). `Ok(None)` when
/// `id` is absent or not an `IfcRecurrencePattern` of that release; IFC2X3
/// declares none.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn recurrence_pattern(
    model: &Model,
    id: EntityId,
) -> Result<Option<Recurrence>, ScheduleReadError> {
    Ok(read_recurrence(ReadRelease::of(model)?, model, id))
}

pub(crate) fn read_recurrence(
    release: ReadRelease,
    model: &Model,
    id: EntityId,
) -> Option<Recurrence> {
    let entity = model.get(id)?;
    if !release.is_a(&entity.type_name, PATTERN) {
        return None;
    }
    let mut time_periods = Vec::new();
    if let Some(value) = release.value(PATTERN, entity, "TimePeriods") {
        value.for_each_ref(&mut |period| {
            if let Some(period) = read_time_period(release, model, period) {
                time_periods.push(period);
            }
        });
    }
    let integer = |attribute| {
        release
            .value(PATTERN, entity, attribute)
            .and_then(|v| v.unwrap_typed().as_i64())
    };
    let integers = |attribute| match release.value(PATTERN, entity, attribute) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| item.unwrap_typed().as_i64())
            .collect(),
        _ => Vec::new(),
    };
    Some(Recurrence {
        id,
        recurrence_type: release
            .token(PATTERN, entity, "RecurrenceType")
            .and_then(RecurrenceType::parse),
        days: integers("DayComponent"),
        weekdays: integers("WeekdayComponent"),
        months: integers("MonthComponent"),
        position: integer("Position"),
        interval: integer("Interval"),
        occurrences: integer("Occurrences"),
        time_periods,
    })
}

fn read_time_period(release: ReadRelease, model: &Model, id: EntityId) -> Option<TimePeriod> {
    let entity = model.get(id)?;
    if !release.is_a(&entity.type_name, PERIOD) {
        return None;
    }
    let text = |attribute| release.text(PERIOD, entity, attribute).map(str::to_string);
    Some(TimePeriod {
        id,
        start_time: text("StartTime"),
        end_time: text("EndTime"),
    })
}

/// Every work calendar in the model, in file order, read against the
/// model's declared release (#234).
///
/// IFC2X3 declares no `IfcWorkCalendar`, so an IFC2X3 model has none.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn read_work_calendars(model: &Model) -> Result<Vec<WorkCalendar<'_>>, ScheduleReadError> {
    Ok(calendars_in(model, ReadRelease::of(model)?))
}

/// Every work calendar in the model, in file order.
///
/// Reads against the model's declared release when the readers can bind
/// it. A header they cannot bind (IFC4X1, IFC4X2, several schemas) is read
/// as IFC4, as this function always did; [`read_work_calendars`] refuses it
/// instead.
#[deprecated(
    note = "reads an unverified or multi-schema header as IFC4; use `read_work_calendars`, which refuses it (#234)"
)]
#[must_use]
pub fn work_calendars(model: &Model) -> Vec<WorkCalendar<'_>> {
    ReadRelease::of(model)
        .or_else(|_| ReadRelease::of_version(SchemaVersion::Ifc4))
        .map_or_else(|_| Vec::new(), |release| calendars_in(model, release))
}

fn calendars_in(model: &Model, release: ReadRelease) -> Vec<WorkCalendar<'_>> {
    if !release.declares(CALENDAR) {
        return Vec::new();
    }
    release
        .instances_of(model, CALENDAR)
        .into_iter()
        .filter_map(|id| {
            Some(WorkCalendar {
                id,
                entity: model.get(id)?,
                release,
            })
        })
        .collect()
}

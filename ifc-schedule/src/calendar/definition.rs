//! `IfcWorkCalendar` and the working/exception times it declares.
//!
//! # Slots, verified against IFC4 EXPRESS
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
//! ```
//!
//! # Working times and exception times are both `IfcWorkTime`
//!
//! They differ only by which slot holds them: slot 6 declares when work
//! happens, slot 7 declares when it does not. Same entity type, opposite
//! meaning -- so a reader that collects "all the IfcWorkTimes" and treats them
//! uniformly turns holidays into working days.

use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcWorkCalendar` slots.
mod slot {
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
mod work_time_slot {
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
mod recurrence_slot {
    /// `RecurrenceType`.
    pub const RECURRENCE_TYPE: usize = 0;
    /// `WeekdayComponent`.
    pub const WEEKDAY_COMPONENT: usize = 2;
    /// `Position`, for `MONTHLY_BY_POSITION` and friends.
    pub const POSITION: usize = 4;
    /// `Interval`.
    pub const INTERVAL: usize = 5;
    /// `Occurrences`.
    pub const OCCURRENCES: usize = 6;
}

/// Whether a period declares work or an exception to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct Recurrence {
    /// The entity.
    pub id: EntityId,
    /// How it repeats.
    pub recurrence_type: Option<RecurrenceType>,
    /// Weekdays it applies to, 1 = Monday through 7 = Sunday.
    pub weekdays: Vec<i64>,
    /// The ordinal position within the period, for positional patterns.
    ///
    /// `MONTHLY_BY_POSITION` uses it as "the 2nd Tuesday"; a negative value
    /// counts from the end of the period.
    pub position: Option<i64>,
    /// The interval between occurrences.
    pub interval: Option<i64>,
    /// How many times it repeats, if bounded.
    pub occurrences: Option<i64>,
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
pub struct WorkTime {
    /// The `IfcWorkTime` entity.
    pub id: EntityId,
    /// Whether this declares work or an exception.
    pub role: WorkTimeRole,
    /// The period name, as authored.
    pub name: Option<String>,
    /// Start date, as authored.
    pub start: Option<String>,
    /// Finish date, as authored.
    pub finish: Option<String>,
    /// The recurrence pattern, if the period repeats.
    pub recurrence: Option<Recurrence>,
}

/// A borrowed view of an `IfcWorkCalendar`.
#[derive(Debug, Clone, Copy)]
pub struct WorkCalendar<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> WorkCalendar<'m> {
    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The calendar name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The user-facing identification code.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(slot::IDENTIFICATION)
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// Periods when work happens.
    #[must_use]
    pub fn working_times(&self, model: &Model) -> Vec<WorkTime> {
        self.times(model, slot::WORKING_TIMES, WorkTimeRole::Working)
    }

    /// Periods when work does not happen, such as holidays.
    #[must_use]
    pub fn exception_times(&self, model: &Model) -> Vec<WorkTime> {
        self.times(model, slot::EXCEPTION_TIMES, WorkTimeRole::Exception)
    }

    fn times(&self, model: &Model, slot: usize, role: WorkTimeRole) -> Vec<WorkTime> {
        let mut refs = Vec::new();
        if let Some(v) = self.entity.attribute(slot) {
            v.for_each_ref(&mut |id| refs.push(id));
        }
        refs.into_iter()
            .filter_map(|id| read_work_time(model, id, role))
            .collect()
    }
}

fn read_work_time(model: &Model, id: EntityId, role: WorkTimeRole) -> Option<WorkTime> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCWORKTIME") {
        return None;
    }
    let recurrence = match entity.attribute(work_time_slot::RECURRENCE_PATTERN) {
        Some(Value::Ref(pattern)) => read_recurrence(model, *pattern),
        _ => None,
    };
    Some(WorkTime {
        id,
        role,
        name: entity.text(work_time_slot::NAME).map(str::to_string),
        start: entity.text(work_time_slot::START).map(str::to_string),
        finish: entity.text(work_time_slot::FINISH).map(str::to_string),
        recurrence,
    })
}

fn read_recurrence(model: &Model, id: EntityId) -> Option<Recurrence> {
    let entity = model.get(id)?;
    if !entity
        .type_name
        .eq_ignore_ascii_case("IFCRECURRENCEPATTERN")
    {
        return None;
    }
    let recurrence_type = match entity.attribute(recurrence_slot::RECURRENCE_TYPE) {
        Some(Value::Enum(token)) => RecurrenceType::parse(token),
        _ => None,
    };
    let mut weekdays = Vec::new();
    if let Some(Value::List(items)) = entity.attribute(recurrence_slot::WEEKDAY_COMPONENT) {
        for item in items {
            if let Some(day) = item.unwrap_typed().as_i64() {
                weekdays.push(day);
            }
        }
    }
    Some(Recurrence {
        id,
        recurrence_type,
        weekdays,
        position: entity
            .attribute(recurrence_slot::POSITION)
            .and_then(|v| v.unwrap_typed().as_i64()),
        interval: entity
            .attribute(recurrence_slot::INTERVAL)
            .and_then(|v| v.unwrap_typed().as_i64()),
        occurrences: entity
            .attribute(recurrence_slot::OCCURRENCES)
            .and_then(|v| v.unwrap_typed().as_i64()),
    })
}

/// Every work calendar in the model, in file order.
#[must_use]
pub fn work_calendars(model: &Model) -> Vec<WorkCalendar<'_>> {
    model
        .of_type("IFCWORKCALENDAR")
        .map(|(id, entity)| WorkCalendar { id, entity })
        .collect()
}

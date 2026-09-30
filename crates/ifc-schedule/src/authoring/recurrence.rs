//! Authoring `IfcRecurrencePattern` (#233).
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 both declare eight attributes:
//!
//! ```text
//! 0 RecurrenceType  1 DayComponent    2 WeekdayComponent
//! 3 MonthComponent  4 Position        5 Interval
//! 6 Occurrences     7 TimePeriods (OPTIONAL LIST [1:?] OF IfcTimePeriod)
//! ```
//!
//! Every record is written with all eight, `TimePeriods` included, so an
//! authored pattern is never a short record. IFC2X3 declares neither
//! `IfcRecurrencePattern` nor `IfcTimePeriod`; [`create_recurrence_pattern_in`]
//! refuses it with `EntityNotInSchema`.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use super::ScheduleAuthoringResult;
use crate::calendar::recurrence_slot;
use crate::error::ScheduleAuthoringError;

const ENTITY: &str = "IFCRECURRENCEPATTERN";
const TIME_PERIOD: &str = "IFCTIMEPERIOD";

/// Authored fields for `IfcRecurrencePattern`.
///
/// Weekday and month components are 1-based in the schema: 1 = Monday
/// through 7 = Sunday, and 1 = January through 12 = December. Out-of-range
/// values are refused rather than written, because a reader has no way to
/// tell a 0-based authoring mistake from a deliberate value.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RecurrenceDraft<'a> {
    /// `RecurrenceType`. Required by the schema.
    pub recurrence_type: &'a str,
    /// `DayComponent`: days of the month, 1..=31.
    pub days: Vec<i64>,
    /// `WeekdayComponent`: 1 = Monday through 7 = Sunday.
    pub weekdays: Vec<i64>,
    /// `MonthComponent`: 1 = January through 12 = December.
    pub months: Vec<i64>,
    /// `Position`, for positional patterns. Negative counts from the end.
    pub position: Option<i64>,
    /// `Interval`: repeat every n periods. Must be positive.
    pub interval: Option<i64>,
    /// `Occurrences`: how many times it repeats. Must be positive.
    pub occurrences: Option<i64>,
    /// `TimePeriods`: the `IfcTimePeriod` records (from
    /// [`create_time_period`](super::create_time_period)) each occurrence
    /// spans, in authored order. Empty writes `$` (#233).
    pub time_periods: Vec<EntityId>,
}

/// Stage an `IfcRecurrencePattern` with all eight IFC4/IFC4X3 attributes.
///
/// Refuses component values outside the schema's 1-based ranges, and a
/// non-positive interval or occurrence count: "every 0 weeks" and "repeats
/// -1 times" both parse and both describe nothing.
///
/// Takes no model, so it cannot see the model's release or resolve the
/// `time_periods` references: it is for IFC4 and IFC4X3 only and writes
/// the references as given. [`create_recurrence_pattern_in`] checks both.
///
/// # Errors
///
/// `InvalidValue` for a blank type, an out-of-range component or a
/// non-positive count. Nothing is staged on an error.
pub fn create_recurrence_pattern(
    tx: &mut Transaction,
    draft: &RecurrenceDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    check_draft(draft)?;
    let mut attributes = vec![Value::Null; recurrence_slot::TIME_PERIODS + 1];
    attributes[recurrence_slot::RECURRENCE_TYPE] = Value::Enum(draft.recurrence_type.into());
    attributes[recurrence_slot::DAY_COMPONENT] = integer_list(&draft.days);
    attributes[recurrence_slot::WEEKDAY_COMPONENT] = integer_list(&draft.weekdays);
    attributes[recurrence_slot::MONTH_COMPONENT] = integer_list(&draft.months);
    attributes[recurrence_slot::POSITION] = draft.position.map_or(Value::Null, Value::Integer);
    attributes[recurrence_slot::INTERVAL] = draft.interval.map_or(Value::Null, Value::Integer);
    attributes[recurrence_slot::OCCURRENCES] =
        draft.occurrences.map_or(Value::Null, Value::Integer);
    attributes[recurrence_slot::TIME_PERIODS] = if draft.time_periods.is_empty() {
        Value::Null
    } else {
        Value::List(draft.time_periods.iter().copied().map(Value::Ref).collect())
    };
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// [`create_recurrence_pattern`] bound to `model`'s declared release (#233).
///
/// IFC4 and IFC4X3 stage exactly what [`create_recurrence_pattern`] stages,
/// once every `time_periods` reference resolves, in the model or staged on
/// `tx`, to an `IfcTimePeriod`. IFC2X3 declares no `IfcRecurrencePattern`
/// and is refused.
///
/// # Errors
///
/// `EntityNotInSchema` for a release without `IfcRecurrencePattern`,
/// `MultipleSchemas` or `UnsupportedSchema` when the header binds no single
/// known release, `MissingReference` or `WrongReferenceType` for a time
/// period that is absent or not an `IfcTimePeriod`, and those of
/// [`create_recurrence_pattern`]. Nothing is staged on an error.
pub fn create_recurrence_pattern_in(
    tx: &mut Transaction,
    model: &Model,
    draft: &RecurrenceDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    crate::release::bind(model)?.require_entity(ENTITY)?;
    check_draft(draft)?;
    for &target in &draft.time_periods {
        let actual = crate::release::projected_type(tx, model, target).ok_or(
            ScheduleAuthoringError::MissingReference {
                entity: ENTITY,
                attribute: "TimePeriods",
                target,
            },
        )?;
        if !actual.eq_ignore_ascii_case(TIME_PERIOD) {
            return Err(ScheduleAuthoringError::WrongReferenceType {
                entity: ENTITY,
                attribute: "TimePeriods",
                target,
                actual,
                expected: TIME_PERIOD,
            });
        }
    }
    create_recurrence_pattern(tx, draft)
}

fn check_draft(draft: &RecurrenceDraft<'_>) -> ScheduleAuthoringResult<()> {
    if draft.recurrence_type.trim().is_empty() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute: "RecurrenceType",
            expected: "a non-empty IfcRecurrenceTypeEnum value",
        });
    }
    check_range(&draft.days, 1, 31, "DayComponent")?;
    check_range(&draft.weekdays, 1, 7, "WeekdayComponent")?;
    check_range(&draft.months, 1, 12, "MonthComponent")?;
    check_positive(draft.interval, "Interval")?;
    check_positive(draft.occurrences, "Occurrences")
}

/// Refuse component values outside the schema's inclusive range.
fn check_range(
    values: &[i64],
    low: i64,
    high: i64,
    attribute: &'static str,
) -> ScheduleAuthoringResult<()> {
    if values.iter().any(|v| !(low..=high).contains(v)) {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute,
            expected: "components within the schema's 1-based range",
        });
    }
    Ok(())
}

/// Refuse a stated count that is zero or negative.
fn check_positive(value: Option<i64>, attribute: &'static str) -> ScheduleAuthoringResult<()> {
    if value.is_some_and(|v| v <= 0) {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute,
            expected: "a positive count",
        });
    }
    Ok(())
}

/// An omitted list stays `Null` rather than becoming an empty aggregate.
fn integer_list(values: &[i64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    Value::List(values.iter().copied().map(Value::Integer).collect())
}

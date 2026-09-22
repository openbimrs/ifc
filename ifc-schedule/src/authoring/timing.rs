//! Authoring task times, recurring task times, and time periods.
//!
//! Split from the module root: these four writers share the
//! `IfcTaskTime` slot table, and the root was over the monolith limit.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use super::{optional_text, ScheduleAuthoringResult};
use crate::error::ScheduleAuthoringError;
use crate::task::definition::time_slot;

/// Authored fields for `IfcTaskTime`.
///
/// Durations are ISO 8601 duration strings and timestamps ISO 8601
/// datetimes, written exactly as given. Parsing them here would mean this
/// crate owning calendar arithmetic it deliberately does not.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskTimeDraft<'a> {
    /// `IfcPhysicalQuantity`-style Name, slot 0.
    pub name: Option<&'a str>,
    /// `IfcTaskTime.DurationType`, `WORKTIME` or `ELAPSEDTIME`.
    pub duration_type: Option<&'a str>,
    /// `IfcTaskTime.ScheduleDuration`, an ISO 8601 duration.
    pub schedule_duration: Option<&'a str>,
    /// `IfcTaskTime.ScheduleStart`.
    pub schedule_start: Option<&'a str>,
    /// `IfcTaskTime.ScheduleFinish`.
    pub schedule_finish: Option<&'a str>,
    /// `IfcTaskTime.ActualStart`.
    pub actual_start: Option<&'a str>,
    /// `IfcTaskTime.ActualFinish`.
    pub actual_finish: Option<&'a str>,
    /// `IfcTaskTime.IsCritical`.
    pub is_critical: Option<bool>,
    /// `IfcTaskTime.Completion`, a percentage in `0..=100`.
    pub completion: Option<f64>,
}

/// Build and validate the shared `IfcTaskTime` attribute vector.
///
/// `IfcTaskTimeRecurring` inherits every one of these slots and adds
/// `Recurrence` at the end, so both writers share this body rather than
/// duplicating nine slot assignments and two range checks.
fn task_time_attributes(
    entity: &'static str,
    draft: TaskTimeDraft<'_>,
) -> ScheduleAuthoringResult<Vec<Value>> {
    if let Some(completion) = draft.completion {
        if !completion.is_finite() || !(0.0..=100.0).contains(&completion) {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity,
                attribute: "Completion",
                expected: "a percentage in 0..=100",
            });
        }
    }
    if let Some(kind) = draft.duration_type {
        if !["WORKTIME", "ELAPSEDTIME", "NOTDEFINED"]
            .iter()
            .any(|known| known.eq_ignore_ascii_case(kind))
        {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity,
                attribute: "DurationType",
                expected: "WORKTIME, ELAPSEDTIME or NOTDEFINED",
            });
        }
    }
    let mut attributes = vec![Value::Null; time_slot::COMPLETION + 1];
    attributes[0] = optional_text(draft.name);
    attributes[time_slot::DURATION_TYPE] = draft
        .duration_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    attributes[time_slot::SCHEDULE_DURATION] = optional_text(draft.schedule_duration);
    attributes[time_slot::SCHEDULE_START] = optional_text(draft.schedule_start);
    attributes[time_slot::SCHEDULE_FINISH] = optional_text(draft.schedule_finish);
    attributes[time_slot::ACTUAL_START] = optional_text(draft.actual_start);
    attributes[time_slot::ACTUAL_FINISH] = optional_text(draft.actual_finish);
    attributes[time_slot::IS_CRITICAL] = draft.is_critical.map_or(Value::Null, Value::Bool);
    attributes[time_slot::COMPLETION] = draft.completion.map_or(Value::Null, Value::Real);
    Ok(attributes)
}

/// Stage an `IfcTaskTime`.
///
/// # Errors
///
/// Refuses a completion outside `0..=100` and an unknown duration type.
pub fn create_task_time(
    tx: &mut Transaction,
    draft: TaskTimeDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    let attributes = task_time_attributes("IFCTASKTIME", draft)?;
    Ok(tx.create(Entity::new("IFCTASKTIME", attributes)))
}

/// Stage an `IfcTaskTimeRecurring`.
///
/// The recurring form is an `IfcTaskTime` plus a required
/// `Recurrence` at slot 20. The pattern is what makes the task repeat,
/// so it is a required argument rather than an optional draft field:
/// a recurring time with no pattern recurs never.
///
/// # Errors
///
/// As for [`create_task_time`], and refuses a `recurrence` that is not
/// an `IfcRecurrencePattern`.
pub fn create_task_time_recurring(
    tx: &mut Transaction,
    model: &Model,
    draft: TaskTimeDraft<'_>,
    recurrence: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCTASKTIMERECURRING";
    const RECURRENCE_SLOT: usize = 20;
    let pattern_is_valid = tx
        .edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            Edit::Create { id, entity } if *id == recurrence => {
                Some(entity.type_name.as_ref().to_owned())
            }
            _ => None,
        })
        .or_else(|| {
            model
                .get(recurrence)
                .map(|entity| entity.type_name.as_ref().to_owned())
        })
        .is_some_and(|name| name.eq_ignore_ascii_case("IFCRECURRENCEPATTERN"));
    if !pattern_is_valid {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute: "Recurrence",
            expected: "an IfcRecurrencePattern",
        });
    }
    let mut attributes = task_time_attributes(ENTITY, draft)?;
    attributes.resize(RECURRENCE_SLOT + 1, Value::Null);
    attributes[RECURRENCE_SLOT] = Value::Ref(recurrence);
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcTimePeriod`: a start and end time of day.
///
/// Both slots are `IfcTime`, a time of day rather than a timestamp,
/// and both are required. The values are written as given for the
/// same reason `IfcTaskTime` does not parse its durations: the codec
/// round-trips text, and reinterpreting it here would lose whatever
/// the authoring tool meant by it.
///
/// # Errors
///
/// Refuses a blank start or end.
pub fn create_time_period(
    tx: &mut Transaction,
    start_time: &str,
    end_time: &str,
) -> ScheduleAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCTIMEPERIOD";
    for (attribute, value) in [("StartTime", start_time), ("EndTime", end_time)] {
        if value.trim().is_empty() {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: ENTITY,
                attribute,
                expected: "a non-empty ISO 8601 time",
            });
        }
    }
    Ok(tx.create(Entity::new(
        ENTITY,
        vec![Value::Text(start_time.into()), Value::Text(end_time.into())],
    )))
}

//! Transactional authoring for tasks, task times and sequence links.
//!
//! The crate could read a programme and not write one. These constructors
//! reuse the slot constants the readers in [`crate::task`] and
//! [`crate::sequence`] already declare, so an authored task is readable by
//! construction rather than by coincidence -- `IfcTask` carries thirteen
//! slots, seven of them inherited from `IfcRoot`, `IfcObject` and
//! `IfcProcess`, and a constructor that counted only the six declared
//! attributes would put Status where GlobalId belongs.
//!
//! Durations and timestamps are written as authored: ISO 8601 strings are
//! not parsed here, because a scheduling tool owns calendar semantics and
//! silently normalising them would lose the authored intent.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::ScheduleAuthoringError;
use crate::sequence::relation::slot as sequence_slot;
use crate::task::definition::{task_slot, time_slot};

/// Result of a schedule authoring call.
pub type ScheduleAuthoringResult<T> = Result<T, ScheduleAuthoringError>;

/// Authored fields for `IfcTask`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskDraft<'a> {
    /// `IfcRoot.GlobalId`. Must be a valid IFC compressed GUID.
    pub global_id: &'a str,
    /// `IfcRoot.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcProcess.Identification`, if given.
    pub identification: Option<&'a str>,
    /// `IfcProcess.LongDescription`, if given.
    pub long_description: Option<&'a str>,
    /// `IfcTask.Status`, if given.
    pub status: Option<&'a str>,
    /// `IfcTask.WorkMethod`, if given.
    pub work_method: Option<&'a str>,
    /// `IfcTask.IsMilestone`. Required by the schema.
    pub is_milestone: bool,
    /// `IfcTask.Priority`, if given. An `IfcInteger` in `0..=100`.
    pub priority: Option<i64>,
    /// `IfcTask.TaskTime`, an `IfcTaskTime` reference, if given.
    pub task_time: Option<EntityId>,
    /// `IfcTask.PredefinedType`, if given.
    pub predefined_type: Option<&'a str>,
}

/// Stage an `IfcTask`.
///
/// Slots are filled by the reader's own constants, so the seven inherited
/// positions are reserved even when unset rather than counted by hand.
pub fn create_task(
    tx: &mut Transaction,
    draft: TaskDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    if Guid::parse(draft.global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCTASK",
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if let Some(priority) = draft.priority {
        if !(0..=100).contains(&priority) {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: "IFCTASK",
                attribute: "Priority",
                expected: "an integer in 0..=100",
            });
        }
    }
    let mut attributes = vec![Value::Null; task_slot::PREDEFINED_TYPE + 1];
    attributes[task_slot::GLOBAL_ID] = Value::Text(draft.global_id.into());
    attributes[task_slot::NAME] = optional_text(draft.name);
    attributes[task_slot::DESCRIPTION] = optional_text(draft.description);
    attributes[task_slot::IDENTIFICATION] = optional_text(draft.identification);
    attributes[task_slot::LONG_DESCRIPTION] = optional_text(draft.long_description);
    attributes[task_slot::STATUS] = optional_text(draft.status);
    attributes[task_slot::WORK_METHOD] = optional_text(draft.work_method);
    attributes[task_slot::IS_MILESTONE] = Value::Bool(draft.is_milestone);
    attributes[task_slot::PRIORITY] = draft.priority.map_or(Value::Null, Value::Integer);
    attributes[task_slot::TASK_TIME] = draft.task_time.map_or(Value::Null, Value::Ref);
    attributes[task_slot::PREDEFINED_TYPE] = draft
        .predefined_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new("IFCTASK", attributes)))
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.into()))
}
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

/// Stage an `IfcTaskTime`.
pub fn create_task_time(
    tx: &mut Transaction,
    draft: TaskTimeDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    if let Some(completion) = draft.completion {
        if !completion.is_finite() || !(0.0..=100.0).contains(&completion) {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: "IFCTASKTIME",
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
                entity: "IFCTASKTIME",
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
    Ok(tx.create(Entity::new("IFCTASKTIME", attributes)))
}

/// Stage an `IfcRelSequence` linking a predecessor to a successor.
///
/// A task may not precede itself: a self-loop is an unsatisfiable
/// constraint that the traversal in [`crate::query`] would otherwise have
/// to detect as a cycle at read time.
pub fn create_sequence(
    tx: &mut Transaction,
    global_id: &str,
    predecessor: EntityId,
    successor: EntityId,
    sequence_type: Option<&str>,
    time_lag: Option<EntityId>,
) -> ScheduleAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELSEQUENCE",
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if predecessor == successor {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELSEQUENCE",
            attribute: "RelatedProcess",
            expected: "a successor distinct from the predecessor",
        });
    }
    let mut attributes = vec![Value::Null; sequence_slot::SEQUENCE_TYPE + 2];
    attributes[0] = Value::Text(global_id.into());
    attributes[sequence_slot::RELATING] = Value::Ref(predecessor);
    attributes[sequence_slot::RELATED] = Value::Ref(successor);
    attributes[sequence_slot::TIME_LAG] = time_lag.map_or(Value::Null, Value::Ref);
    attributes[sequence_slot::SEQUENCE_TYPE] =
        sequence_type.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new("IFCRELSEQUENCE", attributes)))
}

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

use crate::calendar::{work_calendar_slot, work_time_slot};
use crate::error::ScheduleAuthoringError;
use crate::query::{assigns_slot, nests_slot};
use crate::schedule::work_control_slot as control_slot;
use crate::schedule::WorkControlKind;
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

/// Authored fields for `IfcWorkPlan` and `IfcWorkSchedule`.
///
/// Both are `IfcWorkControl` subtypes with identical slots, so one draft
/// serves both and the kind picks the entity type. Timestamps and
/// durations are ISO 8601 strings written exactly as given, for the same
/// reason `IfcTaskTime` does not parse them.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkControlDraft<'a> {
    /// `IfcRoot.GlobalId`. Must be a valid IFC compressed GUID.
    pub global_id: &'a str,
    /// `IfcRoot.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcWorkControl.Identification`, if given.
    pub identification: Option<&'a str>,
    /// `IfcWorkControl.CreationDate`. Required by the schema.
    pub creation_date: &'a str,
    /// `IfcWorkControl.Purpose`, if given.
    pub purpose: Option<&'a str>,
    /// `IfcWorkControl.Duration`, an ISO 8601 duration, if given.
    pub duration: Option<&'a str>,
    /// `IfcWorkControl.TotalFloat`, an ISO 8601 duration, if given.
    pub total_float: Option<&'a str>,
    /// `IfcWorkControl.StartTime`. Required by the schema.
    pub start_time: &'a str,
    /// `IfcWorkControl.FinishTime`, if given.
    pub finish_time: Option<&'a str>,
    /// `IfcWorkControl.PredefinedType`, if given.
    pub predefined_type: Option<&'a str>,
}

/// Stage an `IfcWorkPlan` or `IfcWorkSchedule`.
///
/// `CreationDate` and `StartTime` are required by the schema, so they are
/// plain fields rather than options: a work control without them parses
/// but does not say when the work happens.
///
/// # Errors
///
/// Refuses a malformed GUID, and an empty required timestamp -- a blank
/// `StartTime` writes a schedule that validates and schedules nothing.
pub fn create_work_control(
    tx: &mut Transaction,
    kind: WorkControlKind,
    draft: WorkControlDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    let type_name = match kind {
        WorkControlKind::Plan => "IFCWORKPLAN",
        WorkControlKind::Schedule => "IFCWORKSCHEDULE",
    };
    if Guid::parse(draft.global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: type_name,
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    for (attribute, value) in [
        ("CreationDate", draft.creation_date),
        ("StartTime", draft.start_time),
    ] {
        if value.trim().is_empty() {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: type_name,
                attribute,
                expected: "a non-empty ISO 8601 timestamp",
            });
        }
    }
    let mut attributes = vec![Value::Null; control_slot::PREDEFINED_TYPE + 1];
    attributes[control_slot::GLOBAL_ID] = Value::Text(draft.global_id.into());
    attributes[control_slot::NAME] = optional_text(draft.name);
    attributes[control_slot::DESCRIPTION] = optional_text(draft.description);
    attributes[control_slot::IDENTIFICATION] = optional_text(draft.identification);
    attributes[control_slot::CREATION_DATE] = Value::Text(draft.creation_date.into());
    attributes[control_slot::PURPOSE] = optional_text(draft.purpose);
    attributes[control_slot::DURATION] = optional_text(draft.duration);
    attributes[control_slot::TOTAL_FLOAT] = optional_text(draft.total_float);
    attributes[control_slot::START_TIME] = Value::Text(draft.start_time.into());
    attributes[control_slot::FINISH_TIME] = optional_text(draft.finish_time);
    attributes[control_slot::PREDEFINED_TYPE] = draft
        .predefined_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(type_name, attributes)))
}

/// Stage an `IfcRelAssignsToControl` binding tasks to a work control.
///
/// This is the link `tasks_of_schedule` reads back: a schedule with no
/// assignment owns nothing, however many tasks the file contains.
///
/// # Errors
///
/// Refuses a malformed GUID, and an empty task list -- an assignment
/// relating no objects parses and assigns nothing.
pub fn assign_tasks_to_control(
    tx: &mut Transaction,
    global_id: &str,
    control: EntityId,
    tasks: &[EntityId],
) -> ScheduleAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELASSIGNSTOCONTROL",
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if tasks.is_empty() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELASSIGNSTOCONTROL",
            attribute: "RelatedObjects",
            expected: "at least one assigned object",
        });
    }
    let mut attributes = vec![Value::Null; assigns_slot::RELATING + 1];
    attributes[assigns_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[assigns_slot::RELATED] =
        Value::List(tasks.iter().copied().map(Value::Ref).collect());
    attributes[assigns_slot::RELATING] = Value::Ref(control);
    Ok(tx.create(Entity::new("IFCRELASSIGNSTOCONTROL", attributes)))
}

/// Stage an `IfcRelNests` nesting child tasks under a parent.
///
/// Task breakdown structure: `IfcRelNests` is the ordered parent-child
/// link `subtasks_of` reads, not `IfcRelAggregates`, which nests physical
/// decomposition instead.
///
/// # Errors
///
/// Refuses a malformed GUID, an empty child list, and a parent that also
/// appears among its own children -- a self-nesting task is a cycle the
/// timeline walk cannot terminate on.
pub fn nest_tasks(
    tx: &mut Transaction,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> ScheduleAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELNESTS",
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if children.is_empty() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELNESTS",
            attribute: "RelatedObjects",
            expected: "at least one nested object",
        });
    }
    if children.contains(&parent) {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCRELNESTS",
            attribute: "RelatedObjects",
            expected: "children that do not include the parent",
        });
    }
    let mut attributes = vec![Value::Null; nests_slot::RELATED + 1];
    attributes[nests_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[nests_slot::RELATING] = Value::Ref(parent);
    attributes[nests_slot::RELATED] =
        Value::List(children.iter().copied().map(Value::Ref).collect());
    Ok(tx.create(Entity::new("IFCRELNESTS", attributes)))
}

/// Stage an `IfcWorkTime`.
///
/// One working or exception period inside a calendar. The recurrence
/// pattern is optional: a period with explicit start and finish dates and
/// no pattern is a single block, which is what a one-off shutdown is.
///
/// # Errors
///
/// Refuses an empty name when one is given, since a named period that
/// carries no name reads back as unnamed rather than as authored.
pub fn create_work_time(
    tx: &mut Transaction,
    name: Option<&str>,
    recurrence: Option<EntityId>,
    start: Option<&str>,
    finish: Option<&str>,
) -> ScheduleAuthoringResult<EntityId> {
    if name.is_some_and(|value| value.trim().is_empty()) {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCWORKTIME",
            attribute: "Name",
            expected: "a non-empty name when one is given",
        });
    }
    let mut attributes = vec![Value::Null; work_time_slot::FINISH + 1];
    attributes[work_time_slot::NAME] = optional_text(name);
    attributes[work_time_slot::RECURRENCE_PATTERN] = recurrence.map_or(Value::Null, Value::Ref);
    attributes[work_time_slot::START] = optional_text(start);
    attributes[work_time_slot::FINISH] = optional_text(finish);
    Ok(tx.create(Entity::new("IFCWORKTIME", attributes)))
}

/// Stage an `IfcWorkCalendar`.
///
/// Working times say when work happens; exception times carve holidays
/// out of them. Both are `IfcWorkTime` lists, so the two roles are
/// separate slots rather than a flag on the period.
///
/// # Errors
///
/// Refuses a malformed GUID, and a calendar with neither working nor
/// exception times -- it constrains nothing but reads as a real calendar.
pub fn create_work_calendar(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    working_times: &[EntityId],
    exception_times: &[EntityId],
    predefined_type: Option<&str>,
) -> ScheduleAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCWORKCALENDAR",
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if working_times.is_empty() && exception_times.is_empty() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCWORKCALENDAR",
            attribute: "WorkingTimes",
            expected: "at least one working or exception period",
        });
    }
    let mut attributes = vec![Value::Null; work_calendar_slot::PREDEFINED_TYPE + 1];
    attributes[work_calendar_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[work_calendar_slot::NAME] = optional_text(name);
    attributes[work_calendar_slot::WORKING_TIMES] = reference_list(working_times);
    attributes[work_calendar_slot::EXCEPTION_TIMES] = reference_list(exception_times);
    attributes[work_calendar_slot::PREDEFINED_TYPE] =
        predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new("IFCWORKCALENDAR", attributes)))
}

/// A reference list, or `Null` when empty.
///
/// An empty IFC set is not the same as an absent one: writing `()` where
/// the file means "not stated" reads back as an authored empty set.
fn reference_list(ids: &[EntityId]) -> Value {
    if ids.is_empty() {
        return Value::Null;
    }
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

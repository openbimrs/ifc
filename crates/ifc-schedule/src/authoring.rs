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
//! # Releases (#202)
//!
//! The writers here take no model, so they cannot see the declared release.
//! They write the layout IFC4 ADD2 TC1 and IFC4X3 ADD2 share, with
//! `IfcRoot.OwnerHistory` unset, and are for those releases only. IFC2X3
//! requires `OwnerHistory` and lays several records out differently, so
//! each `IfcRoot` writer has a `*_with_owner_history` variant that binds the
//! model's declared release, takes a caller-supplied `IfcOwnerHistory` and
//! places every attribute by name from that release's table.
//!
//! Durations and timestamps are written as authored: ISO 8601 strings are
//! not parsed here, because a scheduling tool owns calendar semantics and
//! silently normalising them would lose the authored intent.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::calendar::{work_calendar_slot, work_time_slot};
use crate::error::ScheduleAuthoringError;
use crate::event::{event_slot, event_time_slot};
use crate::query::{assigns_slot, nests_slot};
use crate::schedule::work_control_slot as control_slot;
use crate::schedule::WorkControlKind;
use crate::sequence::lag_slot;
use crate::sequence::relation::slot as sequence_slot;
use crate::task::definition::task_slot;

pub use datetime::{CalendarDate, DateTimeValue, LocalTime};

/// Result of a schedule authoring call.
pub type ScheduleAuthoringResult<T> = Result<T, ScheduleAuthoringError>;

/// Authored fields for `IfcTask`.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`create_task_with_owner_history`] there, which binds the release (#202).
pub fn create_task(
    tx: &mut Transaction,
    draft: TaskDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    checks::task(&draft)?;
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

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.into()))
}
/// Stage an `IfcRelSequence` linking a predecessor to a successor.
///
/// A task may not precede itself: a self-loop is an unsatisfiable
/// constraint that the traversal in [`crate::query`] would otherwise have
/// to detect as a cycle at read time.
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`create_sequence_with_owner_history`] there, which binds the release (#202).
pub fn create_sequence(
    tx: &mut Transaction,
    global_id: &str,
    predecessor: EntityId,
    successor: EntityId,
    sequence_type: Option<&str>,
    time_lag: Option<EntityId>,
) -> ScheduleAuthoringResult<EntityId> {
    checks::sequence(global_id, predecessor, successor)?;
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
/// serves both and the kind picks the entity type. In IFC4 and IFC4X3
/// timestamps and durations are ISO 8601 strings written exactly as given,
/// for the same reason `IfcTaskTime` does not parse them; IFC2X3 dates are
/// [`DateTimeValue`] records, which only
/// [`create_work_control_with_owner_history`] writes.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct WorkControlDraft<'a> {
    /// `IfcRoot.GlobalId`. Must be a valid IFC compressed GUID.
    pub global_id: &'a str,
    /// `IfcRoot.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcWorkControl.Identification`, if given.
    pub identification: Option<&'a str>,
    /// `IfcWorkControl.CreationDate`. Required by the schema: ISO 8601
    /// text in IFC4 and IFC4X3, an `IfcDateTimeSelect` record in IFC2X3.
    pub creation_date: DateTimeValue<'a>,
    /// `IfcWorkControl.Purpose`, if given.
    pub purpose: Option<&'a str>,
    /// `IfcWorkControl.Duration`, an ISO 8601 duration, if given.
    pub duration: Option<&'a str>,
    /// `IfcWorkControl.TotalFloat`, an ISO 8601 duration, if given.
    pub total_float: Option<&'a str>,
    /// `IfcWorkControl.StartTime`. Required by the schema, in the same
    /// form as `creation_date`.
    pub start_time: DateTimeValue<'a>,
    /// `IfcWorkControl.FinishTime`, if given, in the same form as
    /// `creation_date`.
    pub finish_time: Option<DateTimeValue<'a>>,
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
/// Refuses a malformed GUID, an empty required timestamp -- a blank
/// `StartTime` writes a schedule that validates and schedules nothing --
/// and a date that is not ISO 8601 text: the IFC2X3 `IfcDateTimeSelect`
/// records are not IFC4 or IFC4X3 values.
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`create_work_control_with_owner_history`] there, which binds the release (#202).
pub fn create_work_control(
    tx: &mut Transaction,
    kind: WorkControlKind,
    draft: WorkControlDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    let type_name = checks::work_control(kind, &draft)?;
    let text = |attribute: &'static str, value: DateTimeValue<'_>| {
        value.text().map(|text| Value::Text(text.into())).ok_or(
            ScheduleAuthoringError::InvalidValue {
                entity: type_name,
                attribute,
                expected: "ISO 8601 text: IfcDateTimeSelect records are IFC2X3 only",
            },
        )
    };
    let creation_date = text("CreationDate", draft.creation_date)?;
    let start_time = text("StartTime", draft.start_time)?;
    let finish_time = match draft.finish_time {
        Some(value) => text("FinishTime", value)?,
        None => Value::Null,
    };
    let mut attributes = vec![Value::Null; control_slot::PREDEFINED_TYPE + 1];
    attributes[control_slot::GLOBAL_ID] = Value::Text(draft.global_id.into());
    attributes[control_slot::NAME] = optional_text(draft.name);
    attributes[control_slot::DESCRIPTION] = optional_text(draft.description);
    attributes[control_slot::IDENTIFICATION] = optional_text(draft.identification);
    attributes[control_slot::CREATION_DATE] = creation_date;
    attributes[control_slot::PURPOSE] = optional_text(draft.purpose);
    attributes[control_slot::DURATION] = optional_text(draft.duration);
    attributes[control_slot::TOTAL_FLOAT] = optional_text(draft.total_float);
    attributes[control_slot::START_TIME] = start_time;
    attributes[control_slot::FINISH_TIME] = finish_time;
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
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`assign_tasks_to_control_with_owner_history`] there, which binds the release (#202).
pub fn assign_tasks_to_control(
    tx: &mut Transaction,
    global_id: &str,
    control: EntityId,
    tasks: &[EntityId],
) -> ScheduleAuthoringResult<EntityId> {
    checks::assignment(global_id, tasks)?;
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
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`nest_tasks_with_owner_history`] there, which binds the release (#202).
pub fn nest_tasks(
    tx: &mut Transaction,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> ScheduleAuthoringResult<EntityId> {
    checks::nesting(global_id, parent, children)?;
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
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`create_work_calendar_with_owner_history`] there, which binds the release (#202).
pub fn create_work_calendar(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    working_times: &[EntityId],
    exception_times: &[EntityId],
    predefined_type: Option<&str>,
) -> ScheduleAuthoringResult<EntityId> {
    checks::calendar(global_id, working_times, exception_times)?;
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
pub(super) fn reference_list(ids: &[EntityId]) -> Value {
    if ids.is_empty() {
        return Value::Null;
    }
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// Authored fields for `IfcEvent`.
///
/// Two schema WHERE rules are enforced here rather than left to a
/// downstream validator: both produce a file that parses cleanly and reads
/// back as a different event than the author meant.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct EventDraft<'a> {
    /// `IfcRoot.GlobalId`. Must be a valid IFC compressed GUID.
    pub global_id: &'a str,
    /// `IfcRoot.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcObject.ObjectType`. Required when `predefined_type` is
    /// `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `IfcProcess.Identification`, if given.
    pub identification: Option<&'a str>,
    /// `IfcProcess.LongDescription`, if given.
    pub long_description: Option<&'a str>,
    /// `IfcEvent.PredefinedType`, if given.
    pub predefined_type: Option<&'a str>,
    /// `IfcEvent.EventTriggerType`, if given.
    pub trigger_type: Option<&'a str>,
    /// `IfcEvent.UserDefinedEventTriggerType`. Required when `trigger_type`
    /// is `USERDEFINED`.
    pub user_defined_trigger_type: Option<&'a str>,
    /// `IfcEvent.EventOccurenceTime`, an `IfcEventTime` reference.
    pub occurence_time: Option<EntityId>,
}

/// Stage an `IfcEvent`.
///
/// Refuses a `USERDEFINED` discriminator whose accompanying label is
/// missing. The schema states both as WHERE rules, and a reader that meets
/// one has no way to recover what the author meant: the event reads back
/// as user-defined with nothing saying what it is.
///
/// `OwnerHistory` is written `$`, which IFC4 and IFC4X3 allow, in their
/// shared layout. This writer takes no model, so it cannot see the declared
/// release: it is for IFC4 and IFC4X3 only. IFC2X3 requires
/// `OwnerHistory` and lays the record out differently; use
/// [`create_event_with_owner_history`] there, which binds the release (#202).
pub fn create_event(
    tx: &mut Transaction,
    draft: EventDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    checks::event(&draft)?;
    let mut attributes = vec![Value::Null; event_slot::EVENT_OCCURENCE_TIME + 1];
    attributes[event_slot::GLOBAL_ID] = Value::Text(draft.global_id.into());
    attributes[event_slot::NAME] = optional_text(draft.name);
    attributes[event_slot::OBJECT_TYPE] = optional_text(draft.object_type);
    attributes[event_slot::IDENTIFICATION] = optional_text(draft.identification);
    attributes[event_slot::LONG_DESCRIPTION] = optional_text(draft.long_description);
    attributes[event_slot::PREDEFINED_TYPE] = optional_enum(draft.predefined_type);
    attributes[event_slot::EVENT_TRIGGER_TYPE] = optional_enum(draft.trigger_type);
    attributes[event_slot::USER_DEFINED_EVENT_TRIGGER_TYPE] =
        optional_text(draft.user_defined_trigger_type);
    attributes[event_slot::EVENT_OCCURENCE_TIME] =
        draft.occurence_time.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCEVENT", attributes)))
}

/// Whether a discriminator names the USERDEFINED case.
pub(super) fn is_user_defined(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.eq_ignore_ascii_case("USERDEFINED"))
}

/// Whether an accompanying label is absent or only whitespace.
///
/// A blank string satisfies EXISTS in the schema but carries no meaning, so
/// it is treated as absent here.
pub(super) fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}

/// An optional enumeration value.
pub(super) fn optional_enum(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Enum(v.into()))
}

/// Authored fields for `IfcEventTime`.
///
/// Dates are ISO 8601 strings written exactly as given, matching how
/// [`create_task_time`] treats its timestamps.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct EventTimeDraft<'a> {
    /// `IfcSchedulingTime.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcEventTime.ActualDate`, if given.
    pub actual: Option<&'a str>,
    /// `IfcEventTime.EarlyDate`, if given.
    pub early: Option<&'a str>,
    /// `IfcEventTime.LateDate`, if given.
    pub late: Option<&'a str>,
    /// `IfcEventTime.ScheduleDate`, if given.
    pub schedule: Option<&'a str>,
}

/// Stage an `IfcEventTime`.
///
/// Every slot is optional in the schema, so an all-empty record is legal
/// and not refused here: it says "the dates are not yet known", which is a
/// meaningful state for a planned event.
pub fn create_event_time(
    tx: &mut Transaction,
    draft: EventTimeDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    let mut attributes = vec![Value::Null; event_time_slot::SCHEDULE_DATE + 1];
    attributes[event_time_slot::NAME] = optional_text(draft.name);
    attributes[event_time_slot::ACTUAL_DATE] = optional_text(draft.actual);
    attributes[event_time_slot::EARLY_DATE] = optional_text(draft.early);
    attributes[event_time_slot::LATE_DATE] = optional_text(draft.late);
    attributes[event_time_slot::SCHEDULE_DATE] = optional_text(draft.schedule);
    Ok(tx.create(Entity::new("IFCEVENTTIME", attributes)))
}

/// Stage an `IfcLagTime`.
///
/// `LagValue` and `DurationType` are both REQUIRED by the schema, unlike
/// every other scheduling-time field, so neither is an `Option` here: a lag
/// that states neither how long nor in what units is not a lag.
///
/// `lag_value` is an `IfcTimeOrRatioSelect`. A duration is an ISO 8601
/// string; a ratio is a plain number. The caller picks, because the two
/// mean different things and this crate will not guess.
///
/// `LagValue` is declared `IfcTimeOrRatioSelect = SELECT (IfcDuration,
/// IfcRatioMeasure)` in IFC4 and IFC4X3, so the value is written as the
/// typed parameter of the member it is (#201): a string as
/// `IFCDURATION('P5D')`, a number as `IFCRATIOMEASURE(0.5)` (an integer is
/// written as that REAL). A value already typed as one of those two members
/// is accepted as is.
///
/// Takes no model, so it cannot see the model's release and is for IFC4 and
/// IFC4X3 only. IFC2X3 declares no `IfcLagTime`: there the lag is
/// `IfcRelSequence.TimeLag` itself, an `IfcTimeMeasure`
/// ([`TimeLag::Seconds`]). To have the header's release checked, use
/// [`create_lag_time_in`].
pub fn create_lag_time(
    tx: &mut Transaction,
    name: Option<&str>,
    lag_value: Value,
    duration_type: &str,
) -> ScheduleAuthoringResult<EntityId> {
    if duration_type.trim().is_empty() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: "IFCLAGTIME",
            attribute: "DurationType",
            expected: "a non-empty IfcTaskDurationEnum value",
        });
    }
    // (member, payload): the SELECT member the value is, as a bare value.
    let (member, payload) = match lag_value {
        Value::Typed { type_name, value } => (Some(type_name), *value),
        bare => (None, bare),
    };
    let is = |name: &str| member.as_ref().is_none_or(|m| m.eq_ignore_ascii_case(name));
    #[allow(clippy::cast_precision_loss)]
    let (member, payload) = match payload {
        Value::Text(text) if is("IFCDURATION") => ("IFCDURATION", Value::Text(text)),
        Value::Real(ratio) if is("IFCRATIOMEASURE") => ("IFCRATIOMEASURE", Value::Real(ratio)),
        Value::Integer(ratio) if is("IFCRATIOMEASURE") => {
            ("IFCRATIOMEASURE", Value::Real(ratio as f64))
        }
        _ => {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: "IFCLAGTIME",
                attribute: "LagValue",
                expected: "a duration string or a ratio number",
            })
        }
    };
    let lag_value = Value::Typed {
        type_name: member.into(),
        value: Box::new(payload),
    };
    let mut attributes = vec![Value::Null; lag_slot::DURATION_TYPE + 1];
    attributes[event_time_slot::NAME] = optional_text(name);
    attributes[lag_slot::LAG_VALUE] = lag_value;
    attributes[lag_slot::DURATION_TYPE] = Value::Enum(duration_type.into());
    Ok(tx.create(Entity::new("IFCLAGTIME", attributes)))
}

/// [`create_lag_time`] bound to `model`'s declared release (#211).
///
/// IFC4 and IFC4X3 are staged exactly as [`create_lag_time`] stages them.
/// IFC2X3 declares no `IfcLagTime` and is refused with `EntityNotInSchema`;
/// author its lag as [`TimeLag::Seconds`] on the sequence instead.
///
/// # Errors
///
/// `EntityNotInSchema` for a release without `IfcLagTime`,
/// `MultipleSchemas` or `UnsupportedSchema` when the header binds no single
/// known release, and those of [`create_lag_time`]. Nothing is staged on an
/// error.
pub fn create_lag_time_in(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    lag_value: Value,
    duration_type: &str,
) -> ScheduleAuthoringResult<EntityId> {
    crate::release::bind(model)?.require_entity("IFCLAGTIME")?;
    create_lag_time(tx, name, lag_value, duration_type)
}

mod builders;
mod checks;
mod datetime;
mod owned;
mod procedure;
mod recurrence;
mod timing;

pub use owned::{
    assign_tasks_to_control_with_owner_history, create_event_with_owner_history,
    create_procedure_with_owner_history, create_sequence_with_owner_history,
    create_task_with_owner_history, create_work_calendar_with_owner_history,
    create_work_control_with_owner_history, nest_tasks_with_owner_history, TimeLag,
};
pub use procedure::{create_procedure, ProcedureDraft};
pub use recurrence::{create_recurrence_pattern, create_recurrence_pattern_in, RecurrenceDraft};
pub use timing::{
    create_task_time, create_task_time_recurring, create_time_period, create_time_period_in,
    TaskTimeDraft,
};

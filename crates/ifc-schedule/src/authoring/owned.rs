//! The `IfcRoot` writers bound to the model's declared release, with a
//! caller-supplied `IfcOwnerHistory` (#202).
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from IFC4
//! on:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! Each writer here runs the draft rules of its plain counterpart, binds
//! the release (`release.rs`), checks that `owner_history` is an
//! `IfcOwnerHistory` in the model or staged on the transaction, and places
//! every attribute by name from the release's table. In IFC4 and IFC4X3 the
//! record is the plain writer's with the reference in the optional slot.
//! One is never invented here: build it with `ifc-author`. This follows
//! `ifc-material` (#77), `ifc-properties` (#191) and `ifc-control` (#198).
//!
//! What IFC2X3 TC1 cannot hold is refused, never dropped or approximated:
//! it declares no `IfcEvent` and no `IfcWorkCalendar`, and its `IfcTask`
//! has no `LongDescription`, `TaskTime` or `PredefinedType` and requires
//! `TaskId`. Where IFC2X3 declares another form, the draft carries both
//! (#214): its `IfcRelSequence.TimeLag` is a required `IfcTimeMeasure`
//! ([`TimeLag::Seconds`]) rather than an `IfcLagTime`
//! ([`TimeLag::LagTime`]), and its `IfcWorkControl` dates are
//! `IfcDateTimeSelect` records ([`DateTimeValue`] record forms, staged here)
//! rather than ISO 8601 text. A form the bound release does not declare is
//! refused with `AuthoringValueType`.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::datetime::{patch, DateTimeValue, PLACEHOLDERS};
use super::procedure::{self, ProcedureDraft};
use super::{
    blank, checks, is_user_defined, optional_enum, optional_text, reference_list, EventDraft,
    ScheduleAuthoringResult, TaskDraft, WorkControlDraft,
};
use crate::error::ScheduleAuthoringError;
use crate::release::{bind, Release};
use crate::schedule::WorkControlKind;
use ifc_schema::SchemaVersion;

/// Lay `values` out in `release`, check the owner history, and stage.
fn stage(
    tx: &mut Transaction,
    model: &Model,
    release: Release,
    entity: &'static str,
    values: Vec<(&'static str, Value)>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let mut named = vec![("OwnerHistory", Value::Ref(owner_history))];
    named.extend(values);
    let record = release.record(entity, named)?;
    release.require_owner_history(tx, model, entity, owner_history)?;
    Ok(tx.create(record))
}

/// `IfcRelSequence.TimeLag`, in the form the release declares.
///
/// ```text
/// IFC2X3_TC1   TimeLag : IfcTimeMeasure;          (REAL, required)
/// IFC4         TimeLag : OPTIONAL IfcLagTime;
/// IFC4X3_ADD2  TimeLag : OPTIONAL IfcLagTime;
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TimeLag {
    /// IFC4 and IFC4X3: an `IfcLagTime` reference, as staged by
    /// [`create_lag_time`](super::create_lag_time).
    LagTime(EntityId),
    /// IFC2X3: an `IfcTimeMeasure`, in seconds.
    Seconds(f64),
}

impl TimeLag {
    fn value(self) -> Value {
        match self {
            Self::LagTime(id) => Value::Ref(id),
            Self::Seconds(seconds) => Value::Real(seconds),
        }
    }
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// [`create_task`](super::create_task) in the model's declared release, with
/// a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// In IFC2X3 `identification` is written as `IfcTask.TaskId`, which that
/// release requires.
///
/// # Errors
///
/// Those of [`create_task`](super::create_task), and:
/// [`ScheduleAuthoringError::MultipleSchemas`] or
/// [`ScheduleAuthoringError::UnsupportedSchema`] when the model binds no
/// single known release; `AuthoringNotInSchema`, `AuthoringValueType` or
/// `AuthoringRequired` for a value the release does not declare, cannot
/// hold, or requires (IFC2X3: `long_description`, `task_time`,
/// `predefined_type`, a missing `identification`);
/// [`ScheduleAuthoringError::MissingReference`] when `owner_history` is
/// neither in the model nor staged; and
/// [`ScheduleAuthoringError::WrongReferenceType`] when it is not an
/// `IfcOwnerHistory`. Nothing is staged on an error.
pub fn create_task_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: TaskDraft<'_>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::task(&draft)?;
    let values = vec![
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("Identification", optional_text(draft.identification)),
        ("LongDescription", optional_text(draft.long_description)),
        ("Status", optional_text(draft.status)),
        ("WorkMethod", optional_text(draft.work_method)),
        ("IsMilestone", Value::Bool(draft.is_milestone)),
        (
            "Priority",
            draft.priority.map_or(Value::Null, Value::Integer),
        ),
        ("TaskTime", draft.task_time.map_or(Value::Null, Value::Ref)),
        ("PredefinedType", optional_enum(draft.predefined_type)),
    ];
    stage(tx, model, release, "IFCTASK", values, owner_history)
}

/// [`create_sequence`](super::create_sequence) in the model's declared
/// release, with a caller-supplied `IfcOwnerHistory`.
///
/// IFC2X3 requires `TimeLag` as an `IfcTimeMeasure` ([`TimeLag::Seconds`])
/// and `SequenceType`; IFC4 and IFC4X3 take an `IfcLagTime`
/// ([`TimeLag::LagTime`]). The other form is refused with
/// `AuthoringValueType`, and a missing required one with
/// `AuthoringRequired`. IFC2X3's `IfcSequenceEnum` has no `USERDEFINED`.
///
/// # Errors
///
/// Those of [`create_sequence`](super::create_sequence), a non-finite
/// [`TimeLag::Seconds`] (`InvalidValue`), and the release, value and
/// owner-history refusals of [`create_task_with_owner_history`]. Nothing is
/// staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn create_sequence_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    predecessor: EntityId,
    successor: EntityId,
    sequence_type: Option<&str>,
    time_lag: Option<TimeLag>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::sequence(global_id, predecessor, successor)?;
    checks::time_lag(time_lag)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingProcess", Value::Ref(predecessor)),
        ("RelatedProcess", Value::Ref(successor)),
        ("TimeLag", time_lag.map_or(Value::Null, TimeLag::value)),
        ("SequenceType", optional_enum(sequence_type)),
    ];
    stage(tx, model, release, "IFCRELSEQUENCE", values, owner_history)
}

/// [`create_work_control`](super::create_work_control) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// IFC2X3 declares `CreationDate`, `StartTime` and `FinishTime` as
/// `IfcDateTimeSelect`: give them as [`DateTimeValue::Date`],
/// [`DateTimeValue::Time`] or [`DateTimeValue::DateAndTime`], and the
/// `IfcCalendarDate`, `IfcLocalTime` and `IfcDateAndTime` records are staged
/// here and referenced (#214). IFC4 and IFC4X3 declare `IfcDateTime` text
/// ([`DateTimeValue::Text`]). The other form is refused with
/// `AuthoringValueType`. In IFC2X3 `identification` is written as the
/// required `Identifier`.
///
/// # Errors
///
/// Those of [`create_work_control`](super::create_work_control) except the
/// refusal of record forms, a record form the schema's rules refuse
/// (`InvalidValue`: a month outside 1..=12, a day the month does not have,
/// an hour, minute or second out of range, a second without a minute), and
/// the release, value and owner-history refusals of
/// [`create_task_with_owner_history`]. Nothing is staged on an error: the
/// date records are staged only once the work control itself is accepted.
pub fn create_work_control_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: WorkControlKind,
    draft: WorkControlDraft<'_>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    let type_name = checks::work_control(kind, &draft)?;
    let dates = [
        Some(draft.creation_date),
        Some(draft.start_time),
        draft.finish_time,
    ];
    let provisional = |slot: usize| {
        dates[slot].map_or(Value::Null, |date: DateTimeValue<'_>| {
            date.provisional(PLACEHOLDERS[slot])
        })
    };
    let values = vec![
        ("OwnerHistory", Value::Ref(owner_history)),
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("Identification", optional_text(draft.identification)),
        ("CreationDate", provisional(0)),
        ("Purpose", optional_text(draft.purpose)),
        ("Duration", optional_text(draft.duration)),
        ("TotalFloat", optional_text(draft.total_float)),
        ("StartTime", provisional(1)),
        ("FinishTime", provisional(2)),
        ("PredefinedType", optional_enum(draft.predefined_type)),
    ];
    // Every refusal before the first edit: the record with placeholders,
    // then the owner history; only then the date records.
    let mut record = release.record(type_name, values)?;
    release.require_owner_history(tx, model, type_name, owner_history)?;
    patch(tx, &mut record, &dates);
    Ok(tx.create(record))
}

/// [`assign_tasks_to_control`](super::assign_tasks_to_control) in the
/// model's declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// `RelatedObjectsType` is left unset, as the plain writer leaves it.
///
/// # Errors
///
/// Those of [`assign_tasks_to_control`](super::assign_tasks_to_control),
/// and the release and owner-history refusals of
/// [`create_task_with_owner_history`]. Nothing is staged on an error.
pub fn assign_tasks_to_control_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    control: EntityId,
    tasks: &[EntityId],
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::assignment(global_id, tasks)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatedObjects", refs(tasks)),
        ("RelatingControl", Value::Ref(control)),
    ];
    stage(
        tx,
        model,
        release,
        "IFCRELASSIGNSTOCONTROL",
        values,
        owner_history,
    )
}

/// [`nest_tasks`](super::nest_tasks) in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`nest_tasks`](super::nest_tasks), and the release and
/// owner-history refusals of [`create_task_with_owner_history`]. Nothing is
/// staged on an error.
pub fn nest_tasks_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::nesting(global_id, parent, children)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingObject", Value::Ref(parent)),
        ("RelatedObjects", refs(children)),
    ];
    stage(tx, model, release, "IFCRELNESTS", values, owner_history)
}

/// [`create_work_calendar`](super::create_work_calendar) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// IFC2X3 declares no `IfcWorkCalendar` and is refused with
/// `EntityNotInSchema`.
///
/// # Errors
///
/// Those of [`create_work_calendar`](super::create_work_calendar), and the
/// release, entity and owner-history refusals of
/// [`create_task_with_owner_history`]. Nothing is staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn create_work_calendar_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    working_times: &[EntityId],
    exception_times: &[EntityId],
    predefined_type: Option<&str>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::calendar(global_id, working_times, exception_times)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", optional_text(name)),
        ("WorkingTimes", reference_list(working_times)),
        ("ExceptionTimes", reference_list(exception_times)),
        ("PredefinedType", optional_enum(predefined_type)),
    ];
    stage(tx, model, release, "IFCWORKCALENDAR", values, owner_history)
}

/// [`create_event`](super::create_event) in the model's declared release,
/// with a caller-supplied `IfcOwnerHistory`.
///
/// IFC2X3 declares no `IfcEvent` and is refused with `EntityNotInSchema`.
///
/// # Errors
///
/// Those of [`create_event`](super::create_event), and the release, entity
/// and owner-history refusals of [`create_task_with_owner_history`].
/// Nothing is staged on an error.
pub fn create_event_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: EventDraft<'_>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    checks::event(&draft)?;
    let values = vec![
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("Name", optional_text(draft.name)),
        ("ObjectType", optional_text(draft.object_type)),
        ("Identification", optional_text(draft.identification)),
        ("LongDescription", optional_text(draft.long_description)),
        ("PredefinedType", optional_enum(draft.predefined_type)),
        ("EventTriggerType", optional_enum(draft.trigger_type)),
        (
            "UserDefinedEventTriggerType",
            optional_text(draft.user_defined_trigger_type),
        ),
        (
            "EventOccurenceTime",
            draft.occurence_time.map_or(Value::Null, Value::Ref),
        ),
    ];
    stage(tx, model, release, "IFCEVENT", values, owner_history)
}

/// [`create_procedure`](super::create_procedure) in the model's declared
/// release, with a caller-supplied `IfcOwnerHistory`.
///
/// In IFC2X3 `identification` is written as `ProcedureID` and
/// `predefined_type` as `ProcedureType`, both required there, and
/// `user_defined_procedure_type` as `UserDefinedProcedureType`, which
/// `IfcProcedure.WR4` requires for `USERDEFINED` (#214). IFC4 and IFC4X3
/// declare no `UserDefinedProcedureType` and refuse a value for it with
/// `AuthoringNotInSchema`.
///
/// # Errors
///
/// Those of [`create_procedure`](super::create_procedure) except its
/// refusal of `user_defined_procedure_type`, an IFC2X3 `USERDEFINED`
/// without a non-blank `user_defined_procedure_type` (`InvalidValue`, WR4),
/// and the release, value and owner-history refusals of
/// [`create_task_with_owner_history`]. Nothing is staged on an error.
pub fn create_procedure_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: ProcedureDraft<'_>,
    owner_history: EntityId,
) -> ScheduleAuthoringResult<EntityId> {
    let release = bind(model)?;
    procedure::check(&draft)?;
    if release.version() == SchemaVersion::Ifc2x3
        && is_user_defined(draft.predefined_type)
        && blank(draft.user_defined_procedure_type)
    {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: procedure::ENTITY,
            attribute: "UserDefinedProcedureType",
            expected: "a label when ProcedureType is USERDEFINED (IFC2X3 WR4)",
        });
    }
    let values = vec![
        ("GlobalId", Value::Text(draft.global_id.into())),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        ("Identification", optional_text(draft.identification)),
        ("LongDescription", optional_text(draft.long_description)),
        ("PredefinedType", optional_enum(draft.predefined_type)),
        (
            "UserDefinedProcedureType",
            optional_text(draft.user_defined_procedure_type),
        ),
    ];
    stage(tx, model, release, procedure::ENTITY, values, owner_history)
}

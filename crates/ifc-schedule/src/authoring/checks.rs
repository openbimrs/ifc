//! Draft validation shared by the plain writers and their
//! `*_with_owner_history` variants, so the two cannot drift apart.

use ifc_model::guid::Guid;
use ifc_model::EntityId;

use super::WorkControlDraft;
use super::{blank, is_user_defined, EventDraft, ScheduleAuthoringResult, TaskDraft};
use crate::error::ScheduleAuthoringError;
use crate::schedule::WorkControlKind;

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    expected: &'static str,
) -> ScheduleAuthoringError {
    ScheduleAuthoringError::InvalidValue {
        entity,
        attribute,
        expected,
    }
}

/// Refuse a `GlobalId` that is not an IFC compressed GUID.
pub(super) fn guid(entity: &'static str, global_id: &str) -> ScheduleAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", "an IFC compressed GUID"));
    }
    Ok(())
}

/// `IfcTask`: a valid GUID and a priority in `0..=100`.
pub(super) fn task(draft: &TaskDraft<'_>) -> ScheduleAuthoringResult<()> {
    guid("IFCTASK", draft.global_id)?;
    if let Some(priority) = draft.priority {
        if !(0..=100).contains(&priority) {
            return Err(invalid("IFCTASK", "Priority", "an integer in 0..=100"));
        }
    }
    Ok(())
}

/// `IfcRelSequence.TimeLag` given as seconds: a finite `IfcTimeMeasure`.
pub(super) fn time_lag(time_lag: Option<super::TimeLag>) -> ScheduleAuthoringResult<()> {
    if let Some(super::TimeLag::Seconds(seconds)) = time_lag {
        if !seconds.is_finite() {
            return Err(invalid(
                "IFCRELSEQUENCE",
                "TimeLag",
                "a finite IfcTimeMeasure in seconds",
            ));
        }
    }
    Ok(())
}

/// `IfcRelSequence`: a valid GUID and no task preceding itself.
pub(super) fn sequence(
    global_id: &str,
    predecessor: EntityId,
    successor: EntityId,
) -> ScheduleAuthoringResult<()> {
    guid("IFCRELSEQUENCE", global_id)?;
    if predecessor == successor {
        return Err(invalid(
            "IFCRELSEQUENCE",
            "RelatedProcess",
            "a successor distinct from the predecessor",
        ));
    }
    Ok(())
}

/// `IfcWorkPlan` or `IfcWorkSchedule`: the entity `kind` stages, after a
/// valid GUID and non-empty required timestamps.
pub(super) fn work_control(
    kind: WorkControlKind,
    draft: &WorkControlDraft<'_>,
) -> ScheduleAuthoringResult<&'static str> {
    let type_name = match kind {
        WorkControlKind::Plan => "IFCWORKPLAN",
        WorkControlKind::Schedule => "IFCWORKSCHEDULE",
    };
    guid(type_name, draft.global_id)?;
    for (attribute, value) in [
        ("CreationDate", draft.creation_date),
        ("StartTime", draft.start_time),
    ] {
        if value.text().is_some_and(|text| text.trim().is_empty()) {
            return Err(invalid(
                type_name,
                attribute,
                "a non-empty ISO 8601 timestamp",
            ));
        }
        value.check()?;
    }
    if let Some(finish) = draft.finish_time {
        finish.check()?;
    }
    Ok(type_name)
}

/// `IfcRelAssignsToControl`: a valid GUID and at least one task.
pub(super) fn assignment(global_id: &str, tasks: &[EntityId]) -> ScheduleAuthoringResult<()> {
    guid("IFCRELASSIGNSTOCONTROL", global_id)?;
    if tasks.is_empty() {
        return Err(invalid(
            "IFCRELASSIGNSTOCONTROL",
            "RelatedObjects",
            "at least one assigned object",
        ));
    }
    Ok(())
}

/// `IfcRelNests`: a valid GUID, at least one child, no self-nesting.
pub(super) fn nesting(
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> ScheduleAuthoringResult<()> {
    guid("IFCRELNESTS", global_id)?;
    if children.is_empty() {
        return Err(invalid(
            "IFCRELNESTS",
            "RelatedObjects",
            "at least one nested object",
        ));
    }
    if children.contains(&parent) {
        return Err(invalid(
            "IFCRELNESTS",
            "RelatedObjects",
            "children that do not include the parent",
        ));
    }
    Ok(())
}

/// `IfcWorkCalendar`: a valid GUID and at least one period.
pub(super) fn calendar(
    global_id: &str,
    working_times: &[EntityId],
    exception_times: &[EntityId],
) -> ScheduleAuthoringResult<()> {
    guid("IFCWORKCALENDAR", global_id)?;
    if working_times.is_empty() && exception_times.is_empty() {
        return Err(invalid(
            "IFCWORKCALENDAR",
            "WorkingTimes",
            "at least one working or exception period",
        ));
    }
    Ok(())
}

/// `IfcEvent`: a valid GUID and both USERDEFINED WHERE rules.
pub(super) fn event(draft: &EventDraft<'_>) -> ScheduleAuthoringResult<()> {
    guid("IFCEVENT", draft.global_id)?;
    if is_user_defined(draft.predefined_type) && blank(draft.object_type) {
        return Err(invalid(
            "IFCEVENT",
            "ObjectType",
            "a label when PredefinedType is USERDEFINED",
        ));
    }
    if is_user_defined(draft.trigger_type) && blank(draft.user_defined_trigger_type) {
        return Err(invalid(
            "IFCEVENT",
            "UserDefinedEventTriggerType",
            "a label when EventTriggerType is USERDEFINED",
        ));
    }
    Ok(())
}

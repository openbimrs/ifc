//! Constructors and setters for the schedule drafts.
//!
//! The drafts are `#[non_exhaustive]` so they can grow a field when a
//! release needs one (#214) without breaking callers: build one with `new`
//! and the setters, each named after the field it sets, as
//! `ifc-resource`'s drafts are built.

use ifc_model::EntityId;

use super::procedure::ProcedureDraft;
use super::timing::TaskTimeDraft;
use super::{
    DateTimeValue, EventDraft, EventTimeDraft, RecurrenceDraft, TaskDraft, WorkControlDraft,
};

impl<'a> TaskDraft<'a> {
    /// Starts a draft for an `IfcTask` with its `GlobalId` and every optional
    /// attribute unset, `IsMilestone` false.
    #[must_use]
    pub fn new(global_id: &'a str) -> Self {
        Self {
            global_id,
            ..Self::default()
        }
    }

    /// Sets `IfcRoot.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `IfcRoot.Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `IfcProcess.Identification` (IFC2X3 `TaskId`).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `IfcProcess.LongDescription`.
    #[must_use]
    pub fn long_description(mut self, value: &'a str) -> Self {
        self.long_description = Some(value);
        self
    }

    /// Sets `IfcTask.Status`.
    #[must_use]
    pub fn status(mut self, value: &'a str) -> Self {
        self.status = Some(value);
        self
    }

    /// Sets `IfcTask.WorkMethod`.
    #[must_use]
    pub fn work_method(mut self, value: &'a str) -> Self {
        self.work_method = Some(value);
        self
    }

    /// Sets `IfcTask.IsMilestone`.
    #[must_use]
    pub fn is_milestone(mut self, value: bool) -> Self {
        self.is_milestone = value;
        self
    }

    /// Sets `IfcTask.Priority`, an integer in `0..=100`.
    #[must_use]
    pub fn priority(mut self, value: i64) -> Self {
        self.priority = Some(value);
        self
    }

    /// Sets `IfcTask.TaskTime`, an `IfcTaskTime` reference.
    #[must_use]
    pub fn task_time(mut self, value: EntityId) -> Self {
        self.task_time = Some(value);
        self
    }

    /// Sets `IfcTask.PredefinedType`.
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }
}

impl<'a> WorkControlDraft<'a> {
    /// Starts a draft for an `IfcWorkPlan` or `IfcWorkSchedule` with its
    /// required `GlobalId`, `CreationDate` and `StartTime`; each date is
    /// ISO 8601 text (`&str`, IFC4 and IFC4X3) or an IFC2X3 record form.
    #[must_use]
    pub fn new(
        global_id: &'a str,
        creation_date: impl Into<DateTimeValue<'a>>,
        start_time: impl Into<DateTimeValue<'a>>,
    ) -> Self {
        Self {
            global_id,
            creation_date: creation_date.into(),
            start_time: start_time.into(),
            ..Self::default()
        }
    }

    /// Sets `IfcRoot.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `IfcRoot.Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `IfcWorkControl.Identification` (IFC2X3 `Identifier`).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `IfcWorkControl.Purpose`.
    #[must_use]
    pub fn purpose(mut self, value: &'a str) -> Self {
        self.purpose = Some(value);
        self
    }

    /// Sets `IfcWorkControl.Duration`, an ISO 8601 duration.
    #[must_use]
    pub fn duration(mut self, value: &'a str) -> Self {
        self.duration = Some(value);
        self
    }

    /// Sets `IfcWorkControl.TotalFloat`, an ISO 8601 duration.
    #[must_use]
    pub fn total_float(mut self, value: &'a str) -> Self {
        self.total_float = Some(value);
        self
    }

    /// Sets `IfcWorkControl.FinishTime`.
    #[must_use]
    pub fn finish_time(mut self, value: impl Into<DateTimeValue<'a>>) -> Self {
        self.finish_time = Some(value.into());
        self
    }

    /// Sets `PredefinedType`.
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }
}

impl<'a> EventDraft<'a> {
    /// Starts a draft for an `IfcEvent` with its `GlobalId` and every optional
    /// attribute unset.
    #[must_use]
    pub fn new(global_id: &'a str) -> Self {
        Self {
            global_id,
            ..Self::default()
        }
    }

    /// Sets `IfcRoot.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `IfcObject.ObjectType`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets `IfcProcess.Identification`.
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `IfcProcess.LongDescription`.
    #[must_use]
    pub fn long_description(mut self, value: &'a str) -> Self {
        self.long_description = Some(value);
        self
    }

    /// Sets `IfcEvent.PredefinedType`.
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }

    /// Sets `IfcEvent.EventTriggerType`.
    #[must_use]
    pub fn trigger_type(mut self, value: &'a str) -> Self {
        self.trigger_type = Some(value);
        self
    }

    /// Sets `IfcEvent.UserDefinedEventTriggerType`.
    #[must_use]
    pub fn user_defined_trigger_type(mut self, value: &'a str) -> Self {
        self.user_defined_trigger_type = Some(value);
        self
    }

    /// Sets `IfcEvent.EventOccurenceTime`, an `IfcEventTime` reference.
    #[must_use]
    pub fn occurence_time(mut self, value: EntityId) -> Self {
        self.occurence_time = Some(value);
        self
    }
}

impl<'a> EventTimeDraft<'a> {
    /// Starts a draft for an `IfcEventTime` with every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `IfcSchedulingTime.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `IfcEventTime.ActualDate`.
    #[must_use]
    pub fn actual(mut self, value: &'a str) -> Self {
        self.actual = Some(value);
        self
    }

    /// Sets `IfcEventTime.EarlyDate`.
    #[must_use]
    pub fn early(mut self, value: &'a str) -> Self {
        self.early = Some(value);
        self
    }

    /// Sets `IfcEventTime.LateDate`.
    #[must_use]
    pub fn late(mut self, value: &'a str) -> Self {
        self.late = Some(value);
        self
    }

    /// Sets `IfcEventTime.ScheduleDate`.
    #[must_use]
    pub fn schedule(mut self, value: &'a str) -> Self {
        self.schedule = Some(value);
        self
    }
}

impl<'a> RecurrenceDraft<'a> {
    /// Starts a draft for an `IfcRecurrencePattern` with its required
    /// `RecurrenceType` and every component unset.
    #[must_use]
    pub fn new(recurrence_type: &'a str) -> Self {
        Self {
            recurrence_type,
            ..Self::default()
        }
    }

    /// Sets `DayComponent`, days of the month in `1..=31`.
    #[must_use]
    pub fn days(mut self, value: Vec<i64>) -> Self {
        self.days = value;
        self
    }

    /// Sets `WeekdayComponent`, 1 = Monday through 7 = Sunday.
    #[must_use]
    pub fn weekdays(mut self, value: Vec<i64>) -> Self {
        self.weekdays = value;
        self
    }

    /// Sets `MonthComponent`, 1 = January through 12 = December.
    #[must_use]
    pub fn months(mut self, value: Vec<i64>) -> Self {
        self.months = value;
        self
    }

    /// Sets `Position`.
    #[must_use]
    pub fn position(mut self, value: i64) -> Self {
        self.position = Some(value);
        self
    }

    /// Sets `Interval`, a positive count.
    #[must_use]
    pub fn interval(mut self, value: i64) -> Self {
        self.interval = Some(value);
        self
    }

    /// Sets `Occurrences`, a positive count.
    #[must_use]
    pub fn occurrences(mut self, value: i64) -> Self {
        self.occurrences = Some(value);
        self
    }
}

impl<'a> ProcedureDraft<'a> {
    /// Starts a draft for an `IfcProcedure` with its `GlobalId` and every optional
    /// attribute unset.
    #[must_use]
    pub fn new(global_id: &'a str) -> Self {
        Self {
            global_id,
            ..Self::default()
        }
    }

    /// Sets `Name`, required by `HasName`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `ObjectType`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets `Identification` (IFC2X3 `ProcedureID`).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `LongDescription`.
    #[must_use]
    pub fn long_description(mut self, value: &'a str) -> Self {
        self.long_description = Some(value);
        self
    }

    /// Sets `PredefinedType` (IFC2X3 `ProcedureType`).
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }

    /// Sets IFC2X3 `UserDefinedProcedureType`.
    #[must_use]
    pub fn user_defined_procedure_type(mut self, value: &'a str) -> Self {
        self.user_defined_procedure_type = Some(value);
        self
    }
}

impl<'a> TaskTimeDraft<'a> {
    /// Starts a draft for an `IfcTaskTime` with every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `DurationType`.
    #[must_use]
    pub fn duration_type(mut self, value: &'a str) -> Self {
        self.duration_type = Some(value);
        self
    }

    /// Sets `ScheduleDuration`, an ISO 8601 duration.
    #[must_use]
    pub fn schedule_duration(mut self, value: &'a str) -> Self {
        self.schedule_duration = Some(value);
        self
    }

    /// Sets `ScheduleStart`.
    #[must_use]
    pub fn schedule_start(mut self, value: &'a str) -> Self {
        self.schedule_start = Some(value);
        self
    }

    /// Sets `ScheduleFinish`.
    #[must_use]
    pub fn schedule_finish(mut self, value: &'a str) -> Self {
        self.schedule_finish = Some(value);
        self
    }

    /// Sets `ActualStart`.
    #[must_use]
    pub fn actual_start(mut self, value: &'a str) -> Self {
        self.actual_start = Some(value);
        self
    }

    /// Sets `ActualFinish`.
    #[must_use]
    pub fn actual_finish(mut self, value: &'a str) -> Self {
        self.actual_finish = Some(value);
        self
    }

    /// Sets `IsCritical`.
    #[must_use]
    pub fn is_critical(mut self, value: bool) -> Self {
        self.is_critical = Some(value);
        self
    }

    /// Sets `Completion`, a percentage in `0..=100`.
    #[must_use]
    pub fn completion(mut self, value: f64) -> Self {
        self.completion = Some(value);
        self
    }
}

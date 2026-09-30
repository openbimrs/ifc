//! IFC2X3 `IfcScheduleTimeControl`, the task time IFC2X3 assigns through
//! `IfcRelAssignsTasks.TimeForTask` (#235).
//!
//! # From IFC2X3 TC1 EXPRESS
//!
//! IFC2X3 declares no `IfcTask.TaskTime` and no `IfcTaskTime`. A task's
//! times are an `IfcScheduleTimeControl`, an `IfcControl` (whose IFC2X3
//! declaration adds no attribute to `IfcObject`), reached through the
//! `IfcRelAssignsTasks` that assigns the task to its work control:
//!
//! ```text
//! IfcRelAssignsTasks  (IfcRelAssignsToControl)
//! 0 GlobalId  1 OwnerHistory  2 Name  3 Description
//! 4 RelatedObjects  5 RelatedObjectsType  6 RelatingControl
//! 7 TimeForTask : OPTIONAL IfcScheduleTimeControl
//!   WR1 exactly one related object, WR2 an IfcTask,
//!   WR3 RelatingControl an IfcWorkControl
//!
//! IfcScheduleTimeControl  (IfcControl)
//! 0 GlobalId  1 OwnerHistory  2 Name  3 Description  4 ObjectType
//! 5 ActualStart  6 EarlyStart  7 LateStart  8 ScheduleStart
//! 9 ActualFinish  10 EarlyFinish  11 LateFinish  12 ScheduleFinish
//! 13 ScheduleDuration  14 ActualDuration  15 RemainingTime
//! 16 FreeFloat  17 TotalFloat  18 IsCritical  19 StatusTime
//! 20 StartFloat  21 FinishFloat  22 Completion
//! ```
//!
//! The dates are `IfcDateTimeSelect` records (`IfcCalendarDate`,
//! `IfcLocalTime`, `IfcDateAndTime`) and the durations `IfcTimeMeasure`
//! numbers in the project's time unit, returned as authored. Every
//! attribute is read by name in the bound release. IFC4 and IFC4X3 declare
//! neither entity, so there a task has no schedule time control.

use ifc_model::{Entity, EntityId, Model, Value};

use super::definition::{Task, TaskTimeAnomaly};
use crate::release::ReadRelease;
use crate::schedule::AuthoredDateTime;
use crate::SchemaVersion;

const ASSIGNS_TASKS: &str = "IFCRELASSIGNSTASKS";
const TIME_CONTROL: &str = "IFCSCHEDULETIMECONTROL";

/// A borrowed view of an IFC2X3 `IfcScheduleTimeControl`, as one
/// `IfcRelAssignsTasks` assigns it to a task (#235).
#[derive(Debug, Clone, Copy)]
pub struct ScheduleTimeControl<'m> {
    id: EntityId,
    entity: &'m Entity,
    assignment: EntityId,
    work_control: Option<EntityId>,
    release: ReadRelease,
}

impl<'m> ScheduleTimeControl<'m> {
    /// The `IfcScheduleTimeControl` entity id.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The `IfcRelAssignsTasks` whose `TimeForTask` this is.
    #[must_use]
    pub fn assignment(&self) -> EntityId {
        self.assignment
    }

    /// That assignment's `RelatingControl`, the work plan or schedule the
    /// task is timed in.
    #[must_use]
    pub fn work_control(&self) -> Option<EntityId> {
        self.work_control
    }

    /// The release this record is read against (IFC2X3).
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn value(&self, attribute: &'static str) -> Option<&'m Value> {
        self.release.value(TIME_CONTROL, self.entity, attribute)
    }

    fn date_time(&self, attribute: &'static str) -> Option<AuthoredDateTime<'m>> {
        match self.value(attribute)? {
            Value::Ref(record) => Some(AuthoredDateTime::Record(*record)),
            value => value.unwrap_typed().as_text().map(AuthoredDateTime::Text),
        }
    }

    fn measure(&self, attribute: &'static str) -> Option<f64> {
        self.value(attribute)?.unwrap_typed().as_f64()
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.release.text(TIME_CONTROL, self.entity, "GlobalId")
    }

    /// The name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.release.text(TIME_CONTROL, self.entity, "Name")
    }

    /// `ActualStart`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn actual_start(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("ActualStart")
    }

    /// `EarlyStart`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn early_start(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("EarlyStart")
    }

    /// `LateStart`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn late_start(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("LateStart")
    }

    /// `ScheduleStart`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn schedule_start(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("ScheduleStart")
    }

    /// `ActualFinish`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn actual_finish(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("ActualFinish")
    }

    /// `EarlyFinish`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn early_finish(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("EarlyFinish")
    }

    /// `LateFinish`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn late_finish(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("LateFinish")
    }

    /// `ScheduleFinish`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn schedule_finish(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("ScheduleFinish")
    }

    /// `StatusTime`, an `IfcDateTimeSelect` record.
    #[must_use]
    pub fn status_time(&self) -> Option<AuthoredDateTime<'m>> {
        self.date_time("StatusTime")
    }

    /// `ScheduleDuration`, an `IfcTimeMeasure` in the project's time unit.
    #[must_use]
    pub fn schedule_duration(&self) -> Option<f64> {
        self.measure("ScheduleDuration")
    }

    /// `ActualDuration`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn actual_duration(&self) -> Option<f64> {
        self.measure("ActualDuration")
    }

    /// `RemainingTime`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn remaining_time(&self) -> Option<f64> {
        self.measure("RemainingTime")
    }

    /// `FreeFloat`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn free_float(&self) -> Option<f64> {
        self.measure("FreeFloat")
    }

    /// `TotalFloat`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn total_float(&self) -> Option<f64> {
        self.measure("TotalFloat")
    }

    /// `StartFloat`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn start_float(&self) -> Option<f64> {
        self.measure("StartFloat")
    }

    /// `FinishFloat`, an `IfcTimeMeasure`.
    #[must_use]
    pub fn finish_float(&self) -> Option<f64> {
        self.measure("FinishFloat")
    }

    /// `IsCritical`, as authored.
    #[must_use]
    pub fn is_critical(&self) -> Option<bool> {
        self.value("IsCritical")?.as_bool()
    }

    /// `Completion`, an `IfcPositiveRatioMeasure`, as authored.
    #[must_use]
    pub fn completion(&self) -> Option<f64> {
        self.measure("Completion")
    }
}

impl<'m> Task<'m> {
    /// The IFC2X3 `IfcScheduleTimeControl`s assigned to this task through
    /// `IfcRelAssignsTasks.TimeForTask`, in file order (#235).
    ///
    /// A task timed in several work controls has one per assignment. Empty
    /// in IFC4 and IFC4X3, which declare neither entity and time tasks
    /// with [`Self::time`]. A `TimeForTask` that references anything but an
    /// `IfcScheduleTimeControl` is reported as
    /// [`TaskTimeAnomaly::NotAScheduleTimeControl`], never read.
    #[must_use]
    pub fn schedule_time_controls(
        &self,
        model: &'m Model,
    ) -> (Vec<ScheduleTimeControl<'m>>, Vec<TaskTimeAnomaly>) {
        let release = self.read_release();
        let mut found = Vec::new();
        let mut anomalies = Vec::new();
        if !release.declares(ASSIGNS_TASKS) {
            return (found, anomalies);
        }
        for assignment in release.instances_of(model, ASSIGNS_TASKS) {
            let Some(relation) = model.get(assignment) else {
                continue;
            };
            let mut related = false;
            if let Some(objects) = release.value(ASSIGNS_TASKS, relation, "RelatedObjects") {
                objects.for_each_ref(&mut |id| related |= id == self.id());
            }
            if !related {
                continue;
            }
            let Some(target) = release.reference(ASSIGNS_TASKS, relation, "TimeForTask") else {
                continue;
            };
            let Some(entity) = model.get(target) else {
                continue;
            };
            if !release.is_a(&entity.type_name, TIME_CONTROL) {
                anomalies.push(TaskTimeAnomaly::NotAScheduleTimeControl {
                    task: self.id(),
                    assignment,
                    target,
                    found: entity.type_name.to_string(),
                });
                continue;
            }
            found.push(ScheduleTimeControl {
                id: target,
                entity,
                assignment,
                work_control: release.reference(ASSIGNS_TASKS, relation, "RelatingControl"),
                release,
            });
        }
        (found, anomalies)
    }
}

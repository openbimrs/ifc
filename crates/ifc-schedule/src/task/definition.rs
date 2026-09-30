//! `IfcTask` and `IfcTaskTime`.
//!
//! # Read by name in the declared release (#212)
//!
//! `IfcTask` is `IfcProcess` -> `IfcObject` -> `IfcRoot`. From the EXPRESS
//! sources, IFC4 ADD2 TC1 and IFC4X3 ADD2 declare thirteen attributes and
//! IFC2X3 TC1 ten:
//!
//! ```text
//! IFC4, IFC4X3  0 GlobalId  1 OwnerHistory  2 Name  3 Description
//!               4 ObjectType  5 Identification  6 LongDescription
//!               7 Status  8 WorkMethod  9 IsMilestone  10 Priority
//!               11 TaskTime  12 PredefinedType
//! IFC2X3        0 GlobalId  1 OwnerHistory  2 Name  3 Description
//!               4 ObjectType  5 TaskId  6 Status  7 WorkMethod
//!               8 IsMilestone  9 Priority
//! ```
//!
//! The IFC4 positions read an IFC2X3 task's status as its long description
//! and its priority as the milestone flag. Every accessor here looks its
//! attribute up by name in the model's declared release instead;
//! `Identification` is IFC2X3's `TaskId`, which IFC4 promoted to
//! `IfcProcess`. What IFC2X3 does not declare (`LongDescription`,
//! `TaskTime`, `PredefinedType`, and `IfcTaskTime` itself) reads as `None`.
//!
//! ```text
//! IfcTaskTime
//! 0 Name                    1 DataOrigin           2 UserDefinedDataOrigin
//! 3 DurationType            4 ScheduleDuration     5 ScheduleStart
//! 6 ScheduleFinish          7 EarlyStart           8 EarlyFinish
//! 9 LateStart              10 LateFinish          11 FreeFloat
//! 12 TotalFloat            13 IsCritical          14 StatusTime
//! 15 ActualDuration        16 ActualStart         17 ActualFinish
//! 18 RemainingTime         19 Completion
//! ```
//!
//! # A milestone has no duration, and that is a rule
//!
//! `IfcTaskTime` carries WHERE rule `WR1`:
//!
//! ```text
//! WR1 : (NOT(EXISTS(SELF\IfcTaskTime.ScheduleDuration))) OR
//!       (NOT(EXISTS(SELF\IfcTaskTime.ScheduleStart))) OR
//!       ... task is not a milestone
//! ```
//!
//! Practically: a task flagged `IsMilestone = .T.` states an instant, not a
//! span, so a stated schedule duration contradicts the flag. This module
//! reports that contradiction rather than choosing which field to believe.

use ifc_model::{Entity, EntityId, Model};

use crate::error::ScheduleReadError;
use crate::release::ReadRelease;
use crate::SchemaVersion;

const TASK: &str = "IFCTASK";
const TASK_TIME: &str = "IFCTASKTIME";

/// `IfcTask` slots in the layout IFC4 and IFC4X3 share, which the
/// modelless `create_task` writes. The reader goes by name.
pub(crate) mod task_slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Description` (from `IfcRoot`).
    pub const DESCRIPTION: usize = 3;
    /// `Identification` (from `IfcProcess`).
    pub const IDENTIFICATION: usize = 5;
    /// `LongDescription` (from `IfcProcess`).
    pub const LONG_DESCRIPTION: usize = 6;
    /// `Status`.
    pub const STATUS: usize = 7;
    /// `WorkMethod`.
    pub const WORK_METHOD: usize = 8;
    /// `IsMilestone`, required.
    pub const IS_MILESTONE: usize = 9;
    /// `Priority`.
    pub const PRIORITY: usize = 10;
    /// `TaskTime`.
    pub const TASK_TIME: usize = 11;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 12;
}

/// `IfcTaskTime` slots, the same in IFC4 and IFC4X3 (IFC2X3 has no
/// `IfcTaskTime`). The writer lays its record out with them; the reader goes
/// by name.
pub(crate) mod time_slot {
    /// `DurationType`, `.WORKTIME.` or `.ELAPSEDTIME.`.
    pub const DURATION_TYPE: usize = 3;
    /// `ScheduleDuration`.
    pub const SCHEDULE_DURATION: usize = 4;
    /// `ScheduleStart`.
    pub const SCHEDULE_START: usize = 5;
    /// `ScheduleFinish`.
    pub const SCHEDULE_FINISH: usize = 6;
    /// `IsCritical`.
    pub const IS_CRITICAL: usize = 13;
    /// `ActualStart`.
    pub const ACTUAL_START: usize = 16;
    /// `ActualFinish`.
    pub const ACTUAL_FINISH: usize = 17;
    /// `Completion`, a percentage.
    pub const COMPLETION: usize = 19;
}

/// Whether a duration counts working time or elapsed time.
///
/// `IfcTaskDurationEnum`. The distinction matters: two days of work time may
/// span four calendar days across a weekend, and a caller converting one to
/// the other needs the calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DurationType {
    /// `.ELAPSEDTIME.`: calendar time, weekends included.
    ElapsedTime,
    /// `.WORKTIME.`: working time as defined by a calendar.
    WorkTime,
    /// `.NOTDEFINED.`
    NotDefined,
}

impl DurationType {
    pub(crate) fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "ELAPSEDTIME" => Self::ElapsedTime,
            "WORKTIME" => Self::WorkTime,
            "NOTDEFINED" => Self::NotDefined,
            _ => return None,
        })
    }
}

/// A contradiction between a task and its stated time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskTimeAnomaly {
    /// The task is a milestone but its time states a schedule duration.
    ///
    /// `IfcTaskTime` `WR1`. A milestone is an instant; a duration contradicts
    /// that, and neither field is authoritative over the other.
    MilestoneWithDuration {
        /// The task.
        task: EntityId,
        /// Its `IfcTaskTime`.
        time: EntityId,
        /// The duration the file states anyway.
        duration: String,
    },
    /// `TaskTime` points at an entity that is not an `IfcTaskTime`.
    NotATaskTime {
        /// The task.
        task: EntityId,
        /// What it points at.
        target: EntityId,
        /// The type actually found.
        found: String,
    },
}

/// A borrowed view of an `IfcTaskTime`.
#[derive(Debug, Clone, Copy)]
pub struct TaskTime<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: ReadRelease,
}

impl<'m> TaskTime<'m> {
    /// The entity id.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Whether the duration is working or elapsed time.
    #[must_use]
    pub fn duration_type(&self) -> Option<DurationType> {
        DurationType::parse(self.release.token(TASK_TIME, self.entity, "DurationType")?)
    }

    /// The planned duration, as an authored ISO 8601 duration.
    #[must_use]
    pub fn schedule_duration(&self) -> Option<&'m str> {
        self.release
            .text(TASK_TIME, self.entity, "ScheduleDuration")
    }

    /// The planned start, as authored.
    #[must_use]
    pub fn schedule_start(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "ScheduleStart")
    }

    /// The planned finish, as authored.
    #[must_use]
    pub fn schedule_finish(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "ScheduleFinish")
    }

    /// The earliest start, as authored.
    #[must_use]
    pub fn early_start(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "EarlyStart")
    }

    /// The latest finish, as authored.
    #[must_use]
    pub fn late_finish(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "LateFinish")
    }

    /// Free float, as an authored ISO 8601 duration.
    #[must_use]
    pub fn free_float(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "FreeFloat")
    }

    /// Total float, as an authored ISO 8601 duration.
    #[must_use]
    pub fn total_float(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "TotalFloat")
    }

    /// Whether the file marks this task as critical.
    ///
    /// Reported as authored: this crate does not compute a critical path,
    /// because doing so needs the full sequence graph and a calendar, and a
    /// computed answer that disagreed with the file would be indistinguishable
    /// from a stated one.
    #[must_use]
    pub fn is_critical(&self) -> Option<bool> {
        self.release
            .value(TASK_TIME, self.entity, "IsCritical")?
            .as_bool()
    }

    /// The actual start, as authored.
    #[must_use]
    pub fn actual_start(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "ActualStart")
    }

    /// The actual finish, as authored.
    #[must_use]
    pub fn actual_finish(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "ActualFinish")
    }

    /// The actual duration, as authored.
    #[must_use]
    pub fn actual_duration(&self) -> Option<&'m str> {
        self.release.text(TASK_TIME, self.entity, "ActualDuration")
    }

    /// Percent complete, if stated.
    #[must_use]
    pub fn completion(&self) -> Option<f64> {
        self.release
            .value(TASK_TIME, self.entity, "Completion")?
            .unwrap_typed()
            .as_f64()
    }
}

/// A borrowed view of an `IfcTask`, read against one release.
#[derive(Debug, Clone, Copy)]
pub struct Task<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: ReadRelease,
}

impl<'m> Task<'m> {
    /// Wrap an entity known to be an `IfcTask`, read against `release`.
    ///
    /// [`tasks`] binds the model's declared release; use this when the
    /// release is known some other way.
    ///
    /// # Errors
    ///
    /// [`ScheduleReadError::UnsupportedSchema`] for a release the readers
    /// are not verified against (IFC4X1, IFC4X2).
    pub fn new(
        id: EntityId,
        entity: &'m Entity,
        release: SchemaVersion,
    ) -> Result<Self, ScheduleReadError> {
        Ok(Self::bound(id, entity, ReadRelease::of_version(release)?))
    }

    pub(crate) const fn bound(id: EntityId, entity: &'m Entity, release: ReadRelease) -> Self {
        Self {
            id,
            entity,
            release,
        }
    }

    /// The release this task is read against.
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version()
    }

    fn text(&self, attribute: &'static str) -> Option<&'m str> {
        self.release.text(TASK, self.entity, attribute)
    }

    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.text("GlobalId")
    }

    /// The task name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// The short description.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.text("Description")
    }

    /// The user-facing identification code, e.g. a WBS number: IFC4's
    /// `Identification`, IFC2X3's `TaskId`.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.text("Identification")
    }

    /// The long description. `None` in IFC2X3, which declares none.
    #[must_use]
    pub fn long_description(&self) -> Option<&'m str> {
        self.text("LongDescription")
    }

    /// The authored status string.
    #[must_use]
    pub fn status(&self) -> Option<&'m str> {
        self.text("Status")
    }

    /// The work method.
    #[must_use]
    pub fn work_method(&self) -> Option<&'m str> {
        self.text("WorkMethod")
    }

    /// Whether the task is a milestone.
    ///
    /// Required by the schema, so `None` means the file omitted a mandatory
    /// field rather than "not a milestone".
    #[must_use]
    pub fn is_milestone(&self) -> Option<bool> {
        self.release
            .value(TASK, self.entity, "IsMilestone")?
            .as_bool()
    }

    /// The scheduling priority, if stated.
    #[must_use]
    pub fn priority(&self) -> Option<i64> {
        self.release.value(TASK, self.entity, "Priority")?.as_i64()
    }

    /// The predefined type token, without its dots. `None` in IFC2X3,
    /// which declares none.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        self.release.token(TASK, self.entity, "PredefinedType")
    }

    /// The id this task's `TaskTime` points at, if any. `None` in IFC2X3,
    /// which declares no `IfcTaskTime`.
    #[must_use]
    pub fn task_time_ref(&self) -> Option<EntityId> {
        self.release.reference(TASK, self.entity, "TaskTime")
    }

    /// Resolve this task's `IfcTaskTime`.
    ///
    /// Returns the view and any anomaly found while resolving it: a reference
    /// to a non-`IfcTaskTime`, or a milestone that states a duration.
    #[must_use]
    pub fn time(&self, model: &'m Model) -> (Option<TaskTime<'m>>, Vec<TaskTimeAnomaly>) {
        let mut anomalies = Vec::new();
        let Some(target) = self.task_time_ref() else {
            return (None, anomalies);
        };
        let Some(entity) = model.get(target) else {
            return (None, anomalies);
        };
        if !entity.type_name.eq_ignore_ascii_case("IFCTASKTIME") {
            anomalies.push(TaskTimeAnomaly::NotATaskTime {
                task: self.id,
                target,
                found: entity.type_name.to_string(),
            });
            return (None, anomalies);
        }

        let time = TaskTime {
            id: target,
            entity,
            release: self.release,
        };
        // WR1: a milestone is an instant, so a schedule duration contradicts
        // the flag. Report both facts; do not pick a winner.
        if self.is_milestone() == Some(true) {
            if let Some(duration) = time.schedule_duration() {
                anomalies.push(TaskTimeAnomaly::MilestoneWithDuration {
                    task: self.id,
                    time: target,
                    duration: duration.to_string(),
                });
            }
        }
        (Some(time), anomalies)
    }
}

/// Every task in the model, in file order, read against the model's
/// declared release.
///
/// # Errors
///
/// [`ScheduleReadError::UnsupportedSchema`] or
/// [`ScheduleReadError::MultipleSchemas`] for a header the readers cannot
/// bind; a header with no schema reads as IFC4.
pub fn tasks(model: &Model) -> Result<Vec<Task<'_>>, ScheduleReadError> {
    let release = ReadRelease::of(model)?;
    Ok(model
        .of_type(TASK)
        .map(|(id, entity)| Task::bound(id, entity, release))
        .collect())
}

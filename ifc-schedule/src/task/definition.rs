//! `IfcTask` and `IfcTaskTime`.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! `IfcTask` is `IfcProcess` -> `IfcObject` -> `IfcRoot`. `IfcProcess`
//! contributes `Identification` and `LongDescription` before the subtype's
//! own fields:
//!
//! ```text
//! 0 GlobalId       1 OwnerHistory     2 Name
//! 3 Description    4 ObjectType       5 Identification    (IfcProcess)
//! 6 LongDescription                   7 Status
//! 8 WorkMethod     9 IsMilestone     10 Priority
//! 11 TaskTime     12 PredefinedType
//! ```
//!
//! `IsMilestone` is slot 9 and REQUIRED -- it is the one non-optional field
//! `IfcTask` adds. `TaskTime` is 11, a reference to `IfcTaskTime`.
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

use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcTask` slots.
mod task_slot {
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

/// `IfcTaskTime` slots.
mod time_slot {
    /// `DurationType`, `.WORKTIME.` or `.ELAPSEDTIME.`.
    pub const DURATION_TYPE: usize = 3;
    /// `ScheduleDuration`.
    pub const SCHEDULE_DURATION: usize = 4;
    /// `ScheduleStart`.
    pub const SCHEDULE_START: usize = 5;
    /// `ScheduleFinish`.
    pub const SCHEDULE_FINISH: usize = 6;
    /// `EarlyStart`.
    pub const EARLY_START: usize = 7;
    /// `LateFinish`.
    pub const LATE_FINISH: usize = 10;
    /// `FreeFloat`.
    pub const FREE_FLOAT: usize = 11;
    /// `TotalFloat`.
    pub const TOTAL_FLOAT: usize = 12;
    /// `IsCritical`.
    pub const IS_CRITICAL: usize = 13;
    /// `ActualDuration`.
    pub const ACTUAL_DURATION: usize = 15;
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
pub enum DurationType {
    /// `.ELAPSEDTIME.`: calendar time, weekends included.
    ElapsedTime,
    /// `.WORKTIME.`: working time as defined by a calendar.
    WorkTime,
    /// `.NOTDEFINED.`
    NotDefined,
}

impl DurationType {
    fn parse(token: &str) -> Option<Self> {
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
        match self.entity.attribute(time_slot::DURATION_TYPE)? {
            Value::Enum(token) => DurationType::parse(token),
            _ => None,
        }
    }

    /// The planned duration, as an authored ISO 8601 duration.
    #[must_use]
    pub fn schedule_duration(&self) -> Option<&'m str> {
        self.entity.text(time_slot::SCHEDULE_DURATION)
    }

    /// The planned start, as authored.
    #[must_use]
    pub fn schedule_start(&self) -> Option<&'m str> {
        self.entity.text(time_slot::SCHEDULE_START)
    }

    /// The planned finish, as authored.
    #[must_use]
    pub fn schedule_finish(&self) -> Option<&'m str> {
        self.entity.text(time_slot::SCHEDULE_FINISH)
    }

    /// The earliest start, as authored.
    #[must_use]
    pub fn early_start(&self) -> Option<&'m str> {
        self.entity.text(time_slot::EARLY_START)
    }

    /// The latest finish, as authored.
    #[must_use]
    pub fn late_finish(&self) -> Option<&'m str> {
        self.entity.text(time_slot::LATE_FINISH)
    }

    /// Free float, as an authored ISO 8601 duration.
    #[must_use]
    pub fn free_float(&self) -> Option<&'m str> {
        self.entity.text(time_slot::FREE_FLOAT)
    }

    /// Total float, as an authored ISO 8601 duration.
    #[must_use]
    pub fn total_float(&self) -> Option<&'m str> {
        self.entity.text(time_slot::TOTAL_FLOAT)
    }

    /// Whether the file marks this task as critical.
    ///
    /// Reported as authored: this crate does not compute a critical path,
    /// because doing so needs the full sequence graph and a calendar, and a
    /// computed answer that disagreed with the file would be indistinguishable
    /// from a stated one.
    #[must_use]
    pub fn is_critical(&self) -> Option<bool> {
        self.entity.attribute(time_slot::IS_CRITICAL)?.as_bool()
    }

    /// The actual start, as authored.
    #[must_use]
    pub fn actual_start(&self) -> Option<&'m str> {
        self.entity.text(time_slot::ACTUAL_START)
    }

    /// The actual finish, as authored.
    #[must_use]
    pub fn actual_finish(&self) -> Option<&'m str> {
        self.entity.text(time_slot::ACTUAL_FINISH)
    }

    /// The actual duration, as authored.
    #[must_use]
    pub fn actual_duration(&self) -> Option<&'m str> {
        self.entity.text(time_slot::ACTUAL_DURATION)
    }

    /// Percent complete, if stated.
    #[must_use]
    pub fn completion(&self) -> Option<f64> {
        self.entity
            .attribute(time_slot::COMPLETION)?
            .unwrap_typed()
            .as_f64()
    }
}

/// A borrowed view of an `IfcTask`.
#[derive(Debug, Clone, Copy)]
pub struct Task<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> Task<'m> {
    /// Wrap an entity known to be an `IfcTask`.
    #[must_use]
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The `GlobalId` string.
    #[must_use]
    pub fn global_id(&self) -> Option<&'m str> {
        self.entity.text(task_slot::GLOBAL_ID)
    }

    /// The task name.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(task_slot::NAME)
    }

    /// The short description.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.entity.text(task_slot::DESCRIPTION)
    }

    /// The user-facing identification code, e.g. a WBS number.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(task_slot::IDENTIFICATION)
    }

    /// The long description.
    #[must_use]
    pub fn long_description(&self) -> Option<&'m str> {
        self.entity.text(task_slot::LONG_DESCRIPTION)
    }

    /// The authored status string.
    #[must_use]
    pub fn status(&self) -> Option<&'m str> {
        self.entity.text(task_slot::STATUS)
    }

    /// The work method.
    #[must_use]
    pub fn work_method(&self) -> Option<&'m str> {
        self.entity.text(task_slot::WORK_METHOD)
    }

    /// Whether the task is a milestone.
    ///
    /// Required by the schema, so `None` means the file omitted a mandatory
    /// field rather than "not a milestone".
    #[must_use]
    pub fn is_milestone(&self) -> Option<bool> {
        self.entity.attribute(task_slot::IS_MILESTONE)?.as_bool()
    }

    /// The scheduling priority, if stated.
    #[must_use]
    pub fn priority(&self) -> Option<i64> {
        self.entity.attribute(task_slot::PRIORITY)?.as_i64()
    }

    /// The predefined type token, without its dots.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(task_slot::PREDEFINED_TYPE)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// The id this task's `TaskTime` slot points at, if any.
    #[must_use]
    pub fn task_time_ref(&self) -> Option<EntityId> {
        match self.entity.attribute(task_slot::TASK_TIME)? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
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

        let time = TaskTime { id: target, entity };
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

/// Every task in the model, in file order.
#[must_use]
pub fn tasks(model: &Model) -> Vec<Task<'_>> {
    model
        .of_type("IFCTASK")
        .map(|(id, entity)| Task::new(id, entity))
        .collect()
}

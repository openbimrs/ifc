//! Authored IFC4 `IfcResourceTime` values.

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
/// Borrowed projection of `IfcResourceTime`.
pub struct ResourceTime<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ResourceTime<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of the projected `IfcResourceTime`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `DataOrigin` enumeration, when authored.
    pub fn data_origin(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_enum("DataOrigin")
    }

    /// The `ScheduleWork` duration, when authored.
    pub fn schedule_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleWork")
    }

    /// The `ScheduleUsage` ratio, when authored.
    pub fn schedule_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("ScheduleUsage")
    }

    /// The `ScheduleStart` date-time, when authored.
    pub fn schedule_start(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleStart")
    }

    /// The `ScheduleFinish` date-time, when authored.
    pub fn schedule_finish(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleFinish")
    }

    /// The `LevelingDelay` duration, when authored.
    pub fn leveling_delay(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LevelingDelay")
    }

    /// The `IsOverAllocated` flag, when authored.
    pub fn is_over_allocated(&self) -> ResourceResult<Option<bool>> {
        self.record.optional_bool("IsOverAllocated")
    }

    /// The `ActualWork` duration, when authored.
    pub fn actual_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ActualWork")
    }

    /// The `ActualUsage` ratio, when authored.
    pub fn actual_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("ActualUsage")
    }

    /// The `RemainingWork` duration, when authored.
    pub fn remaining_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("RemainingWork")
    }

    /// The `RemainingUsage` ratio, when authored.
    pub fn remaining_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("RemainingUsage")
    }

    /// The `Completion` percentage, when authored.
    pub fn completion(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("Completion")
    }
}

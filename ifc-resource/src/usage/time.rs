//! Authored IFC4 `IfcResourceTime` values.

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct ResourceTime<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ResourceTime<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn data_origin(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_enum("DataOrigin")
    }

    pub fn schedule_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleWork")
    }

    pub fn schedule_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("ScheduleUsage")
    }

    pub fn schedule_start(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleStart")
    }

    pub fn schedule_finish(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ScheduleFinish")
    }

    pub fn leveling_delay(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("LevelingDelay")
    }

    pub fn is_over_allocated(&self) -> ResourceResult<Option<bool>> {
        self.record.optional_bool("IsOverAllocated")
    }

    pub fn actual_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("ActualWork")
    }

    pub fn actual_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("ActualUsage")
    }

    pub fn remaining_work(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("RemainingWork")
    }

    pub fn remaining_usage(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("RemainingUsage")
    }

    pub fn completion(&self) -> ResourceResult<Option<f64>> {
        self.record.optional_positive_number("Completion")
    }
}

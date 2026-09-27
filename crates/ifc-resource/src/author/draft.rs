//! Authoring input types for the bounded IFC4 resource slice.

use ifc_model::EntityId;

use crate::ResourceKind;

/// Staged fields for authoring a concrete `IfcConstructionResource`
/// occurrence before it is committed through `ResourceEditor`.
#[derive(Debug, Clone)]
pub struct ResourceDraft<'a> {
    pub(crate) kind: ResourceKind,
    pub(crate) global_id: &'a str,
    pub(crate) name: Option<&'a str>,
    pub(crate) identification: Option<&'a str>,
    pub(crate) long_description: Option<&'a str>,
    pub(crate) object_type: Option<&'a str>,
    pub(crate) usage: Option<EntityId>,
    pub(crate) base_costs: Vec<EntityId>,
    pub(crate) base_quantity: Option<EntityId>,
    pub(crate) predefined_type: Option<&'a str>,
}

impl<'a> ResourceDraft<'a> {
    /// Starts a draft for the given resource subtype and `GlobalId`.
    #[must_use]
    pub fn new(kind: ResourceKind, global_id: &'a str) -> Self {
        Self {
            kind,
            global_id,
            name: None,
            identification: None,
            long_description: None,
            object_type: None,
            usage: None,
            base_costs: Vec::new(),
            base_quantity: None,
            predefined_type: None,
        }
    }

    /// Sets the `Name` attribute.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the `Identification` attribute.
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets the `LongDescription` attribute.
    #[must_use]
    pub fn long_description(mut self, value: &'a str) -> Self {
        self.long_description = Some(value);
        self
    }

    /// Sets the `ObjectType` attribute; required when `predefined_type` is
    /// `USERDEFINED`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets the `Usage` attribute: a reference to an authored
    /// `IfcResourceTime`.
    #[must_use]
    pub fn usage(mut self, value: EntityId) -> Self {
        self.usage = Some(value);
        self
    }

    /// Sets the `BaseCosts` attribute: applied-value references.
    #[must_use]
    pub fn base_costs(mut self, values: Vec<EntityId>) -> Self {
        self.base_costs = values;
        self
    }

    /// Sets the `BaseQuantity` attribute: a physical-quantity reference.
    #[must_use]
    pub fn base_quantity(mut self, value: EntityId) -> Self {
        self.base_quantity = Some(value);
        self
    }

    /// Sets the `PredefinedType` enumeration.
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }
}

/// Staged fields for authoring an `IfcResourceTime` before it is committed
/// through `ResourceEditor`.
#[derive(Debug, Clone, Default)]
pub struct ResourceTimeDraft<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) schedule_work: Option<&'a str>,
    pub(crate) schedule_usage: Option<f64>,
    pub(crate) schedule_start: Option<&'a str>,
    pub(crate) schedule_finish: Option<&'a str>,
    pub(crate) is_over_allocated: Option<bool>,
    pub(crate) status_time: Option<&'a str>,
    pub(crate) actual_work: Option<&'a str>,
    pub(crate) actual_usage: Option<f64>,
    pub(crate) actual_start: Option<&'a str>,
    pub(crate) actual_finish: Option<&'a str>,
    pub(crate) remaining_work: Option<&'a str>,
    pub(crate) remaining_usage: Option<f64>,
    pub(crate) completion: Option<f64>,
}

impl<'a> ResourceTimeDraft<'a> {
    /// Starts an empty draft with every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `Name` attribute.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the `ScheduleWork` duration.
    #[must_use]
    pub fn schedule_work(mut self, value: &'a str) -> Self {
        self.schedule_work = Some(value);
        self
    }

    /// Sets the `ScheduleUsage` ratio.
    #[must_use]
    pub fn schedule_usage(mut self, value: f64) -> Self {
        self.schedule_usage = Some(value);
        self
    }

    /// Sets the `ScheduleStart` date-time.
    #[must_use]
    pub fn schedule_start(mut self, value: &'a str) -> Self {
        self.schedule_start = Some(value);
        self
    }

    /// Sets the `ScheduleFinish` date-time.
    #[must_use]
    pub fn schedule_finish(mut self, value: &'a str) -> Self {
        self.schedule_finish = Some(value);
        self
    }

    /// Sets the `IsOverAllocated` flag.
    #[must_use]
    pub fn is_over_allocated(mut self, value: bool) -> Self {
        self.is_over_allocated = Some(value);
        self
    }

    /// Sets the `StatusTime` date-time.
    #[must_use]
    pub fn status_time(mut self, value: &'a str) -> Self {
        self.status_time = Some(value);
        self
    }

    /// Sets the `ActualWork` duration.
    #[must_use]
    pub fn actual_work(mut self, value: &'a str) -> Self {
        self.actual_work = Some(value);
        self
    }

    /// Sets the `ActualUsage` ratio.
    #[must_use]
    pub fn actual_usage(mut self, value: f64) -> Self {
        self.actual_usage = Some(value);
        self
    }

    /// Sets the `ActualStart` date-time.
    #[must_use]
    pub fn actual_start(mut self, value: &'a str) -> Self {
        self.actual_start = Some(value);
        self
    }

    /// Sets the `ActualFinish` date-time.
    #[must_use]
    pub fn actual_finish(mut self, value: &'a str) -> Self {
        self.actual_finish = Some(value);
        self
    }

    /// Sets the `RemainingWork` duration.
    #[must_use]
    pub fn remaining_work(mut self, value: &'a str) -> Self {
        self.remaining_work = Some(value);
        self
    }

    /// Sets the `RemainingUsage` ratio.
    #[must_use]
    pub fn remaining_usage(mut self, value: f64) -> Self {
        self.remaining_usage = Some(value);
        self
    }

    /// Sets the `Completion` percentage.
    #[must_use]
    pub fn completion(mut self, value: f64) -> Self {
        self.completion = Some(value);
        self
    }
}

/// Staged fields for authoring an `IfcRelAssignsToResource` relation before
/// it is committed through `ResourceEditor`.
#[derive(Debug, Clone)]
pub struct AllocationDraft<'a> {
    pub(crate) global_id: &'a str,
    pub(crate) name: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) resource: EntityId,
    pub(crate) related_objects: Vec<EntityId>,
    pub(crate) related_objects_type: Option<&'a str>,
}

impl<'a> AllocationDraft<'a> {
    /// Starts a draft assigning `related_objects` to `resource`.
    #[must_use]
    pub fn new(global_id: &'a str, resource: EntityId, related_objects: Vec<EntityId>) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            resource,
            related_objects,
            related_objects_type: None,
        }
    }

    /// Sets the `Name` attribute.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the `Description` attribute.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets the `RelatedObjectsType` enumeration.
    #[must_use]
    pub fn related_objects_type(mut self, value: &'a str) -> Self {
        self.related_objects_type = Some(value);
        self
    }
}

/// Staged fields for authoring an `IfcRelNests` relation that nests
/// resources under a parent before it is committed through
/// `ResourceEditor`.
#[derive(Debug, Clone)]
pub struct NestingDraft<'a> {
    pub(crate) global_id: &'a str,
    pub(crate) name: Option<&'a str>,
    pub(crate) description: Option<&'a str>,
    pub(crate) parent: EntityId,
    pub(crate) children: Vec<EntityId>,
}

impl<'a> NestingDraft<'a> {
    /// Starts a draft nesting `children` under `parent`.
    #[must_use]
    pub fn new(global_id: &'a str, parent: EntityId, children: Vec<EntityId>) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            parent,
            children,
        }
    }

    /// Sets the `Name` attribute.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the `Description` attribute.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

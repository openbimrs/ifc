//! Domain-shaped drafts for the bounded IFC4 cost authoring slice.

use ifc_model::EntityId;

use super::datetime::DateTimeValue;
use crate::ArithmeticOperator;

/// Supported `IfcCostItemTypeEnum` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostItemType {
    /// A user-defined kind; `ObjectType` is then required.
    UserDefined,
    /// No more specific predefined kind is asserted.
    NotDefined,
}
impl CostItemType {
    pub(crate) const fn token(self) -> &'static str {
        match self {
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }
}

/// Supported `IfcCostScheduleTypeEnum` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CostScheduleType {
    /// A budget.
    Budget,
    /// A cost plan.
    CostPlan,
    /// An estimate.
    Estimate,
    /// A tender.
    Tender,
    /// A priced bill of quantities.
    PricedBillOfQuantities,
    /// An unpriced bill of quantities.
    UnpricedBillOfQuantities,
    /// A schedule of rates.
    ScheduleOfRates,
    /// A user-defined kind; `ObjectType` is then required.
    UserDefined,
    /// No more specific predefined kind is asserted.
    NotDefined,
}
impl CostScheduleType {
    pub(crate) const fn token(self) -> &'static str {
        match self {
            Self::Budget => "BUDGET",
            Self::CostPlan => "COSTPLAN",
            Self::Estimate => "ESTIMATE",
            Self::Tender => "TENDER",
            Self::PricedBillOfQuantities => "PRICEDBILLOFQUANTITIES",
            Self::UnpricedBillOfQuantities => "UNPRICEDBILLOFQUANTITIES",
            Self::ScheduleOfRates => "SCHEDULEOFRATES",
            Self::UserDefined => "USERDEFINED",
            Self::NotDefined => "NOTDEFINED",
        }
    }
}

/// The unambiguous applied-value shape supported by bounded authoring.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum CostValueKind<'a> {
    /// One finite `IfcMonetaryMeasure`.
    Monetary(f64),
    /// An ordered composition of already existing or staged cost values.
    Components {
        /// Arithmetic operation applied by consumers.
        operator: ArithmeticOperator,
        /// Ordered `IfcCostValue` references; at least one is required.
        components: &'a [EntityId],
    },
}

#[derive(Debug, Clone, Copy)]
/// Draft for one supported `IfcCostValue`.
#[non_exhaustive]
pub struct CostValueDraft<'a> {
    /// Optional display name.
    pub name: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional IFC date lexical value from which the value applies.
    pub applicable_date: Option<&'a str>,
    /// Optional IFC date lexical value through which the value is fixed.
    pub fixed_until_date: Option<&'a str>,
    /// Optional cost category.
    pub category: Option<&'a str>,
    /// Optional applicability condition.
    pub condition: Option<&'a str>,
    /// Supported scalar or composed value shape.
    pub kind: CostValueKind<'a>,
}
impl CostValueDraft<'_> {
    #[must_use]
    /// Create a scalar monetary draft with no optional metadata.
    pub fn monetary(amount: f64) -> Self {
        Self {
            kind: CostValueKind::Monetary(amount),
            ..Self::default()
        }
    }
}
impl Default for CostValueDraft<'_> {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            applicable_date: None,
            fixed_until_date: None,
            category: None,
            condition: None,
            kind: CostValueKind::Monetary(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Draft for a selected IFC4 `IfcCostItem`.
#[non_exhaustive]
pub struct CostItemDraft<'a> {
    /// Compressed IFC `GlobalId`; validated before staging.
    pub global_id: &'a str,
    /// Optional display name.
    pub name: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional user-defined object type.
    pub object_type: Option<&'a str>,
    /// Optional domain identifier.
    pub identification: Option<&'a str>,
    /// Optional bounded item type.
    pub predefined_type: Option<CostItemType>,
    /// Ordered existing or earlier-staged `IfcCostValue` references.
    pub cost_values: &'a [EntityId],
}
#[derive(Debug, Clone, Copy, Default)]
/// Draft for an IFC4 `IfcCostSchedule`.
#[non_exhaustive]
pub struct CostScheduleDraft<'a> {
    /// Compressed IFC `GlobalId`; validated before staging.
    pub global_id: &'a str,
    /// Optional display name.
    pub name: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional user-defined object type.
    pub object_type: Option<&'a str>,
    /// Optional domain identifier.
    pub identification: Option<&'a str>,
    /// Optional bounded schedule type.
    pub predefined_type: Option<CostScheduleType>,
    /// Optional schedule status.
    pub status: Option<&'a str>,
    /// Optional submission date: ISO 8601 `IfcDateTime` text in IFC4 and
    /// IFC4X3, an `IfcDateTimeSelect` record form in IFC2X3.
    pub submitted_on: Option<DateTimeValue<'a>>,
    /// Optional update date, in the same form as `submitted_on`.
    pub update_date: Option<DateTimeValue<'a>>,
}
#[derive(Debug, Clone, Copy)]
/// Draft for ordered cost-item nesting through `IfcRelNests`.
#[non_exhaustive]
pub struct NestingDraft<'a> {
    /// Compressed IFC `GlobalId`; validated before staging.
    pub global_id: &'a str,
    /// Parent `IfcCostItem`.
    pub parent: EntityId,
    /// Non-empty, duplicate-free ordered child list.
    pub children: &'a [EntityId],
}
#[derive(Debug, Clone, Copy)]
/// Draft assigning cost items to a cost schedule.
#[non_exhaustive]
pub struct ScheduleAssignmentDraft<'a> {
    /// Compressed IFC `GlobalId`; validated before staging.
    pub global_id: &'a str,
    /// Relating `IfcCostSchedule`.
    pub schedule: EntityId,
    /// Non-empty, duplicate-free related `IfcCostItem` set.
    pub items: &'a [EntityId],
}

impl<'a> CostValueDraft<'a> {
    /// Sets the display name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets the IFC date from which the value applies.
    #[must_use]
    pub fn applicable_date(mut self, value: &'a str) -> Self {
        self.applicable_date = Some(value);
        self
    }

    /// Sets the IFC date through which the value is fixed.
    #[must_use]
    pub fn fixed_until_date(mut self, value: &'a str) -> Self {
        self.fixed_until_date = Some(value);
        self
    }

    /// Sets the cost category.
    #[must_use]
    pub fn category(mut self, value: &'a str) -> Self {
        self.category = Some(value);
        self
    }

    /// Sets the applicability condition.
    #[must_use]
    pub fn condition(mut self, value: &'a str) -> Self {
        self.condition = Some(value);
        self
    }

    /// Sets the scalar or composed value shape.
    #[must_use]
    pub fn kind(mut self, value: CostValueKind<'a>) -> Self {
        self.kind = value;
        self
    }
}

impl<'a> CostItemDraft<'a> {
    /// Starts a draft for an `IfcCostItem` with its `GlobalId` and every optional
    /// attribute unset.
    #[must_use]
    pub fn new(global_id: &'a str) -> Self {
        Self {
            global_id,
            ..Self::default()
        }
    }

    /// Sets the display name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets the user-defined object type.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets the domain identifier.
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets the item type.
    #[must_use]
    pub fn predefined_type(mut self, value: CostItemType) -> Self {
        self.predefined_type = Some(value);
        self
    }

    /// Sets the ordered `IfcCostValue` references.
    #[must_use]
    pub fn cost_values(mut self, value: &'a [EntityId]) -> Self {
        self.cost_values = value;
        self
    }
}

impl<'a> CostScheduleDraft<'a> {
    /// Starts a draft for an `IfcCostSchedule` with its `GlobalId` and every optional
    /// attribute unset.
    #[must_use]
    pub fn new(global_id: &'a str) -> Self {
        Self {
            global_id,
            ..Self::default()
        }
    }

    /// Sets the display name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets the user-defined object type.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets the domain identifier (IFC2X3 `ID`, required there).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets the schedule type.
    #[must_use]
    pub fn predefined_type(mut self, value: CostScheduleType) -> Self {
        self.predefined_type = Some(value);
        self
    }

    /// Sets the schedule status.
    #[must_use]
    pub fn status(mut self, value: &'a str) -> Self {
        self.status = Some(value);
        self
    }

    /// Sets the submission date.
    #[must_use]
    pub fn submitted_on(mut self, value: impl Into<DateTimeValue<'a>>) -> Self {
        self.submitted_on = Some(value.into());
        self
    }

    /// Sets the update date.
    #[must_use]
    pub fn update_date(mut self, value: impl Into<DateTimeValue<'a>>) -> Self {
        self.update_date = Some(value.into());
        self
    }
}

impl<'a> NestingDraft<'a> {
    /// Starts a draft nesting `children`, in order, under `parent`.
    #[must_use]
    pub const fn new(global_id: &'a str, parent: EntityId, children: &'a [EntityId]) -> Self {
        Self {
            global_id,
            parent,
            children,
        }
    }
}

impl<'a> ScheduleAssignmentDraft<'a> {
    /// Starts a draft assigning `items` to `schedule`.
    #[must_use]
    pub const fn new(global_id: &'a str, schedule: EntityId, items: &'a [EntityId]) -> Self {
        Self {
            global_id,
            schedule,
            items,
        }
    }
}

//! Domain-shaped drafts for the bounded IFC4 cost authoring slice.

use ifc_model::EntityId;

use crate::ArithmeticOperator;

/// Supported `IfcCostItemTypeEnum` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Optional IFC date-time lexical submission value.
    pub submitted_on: Option<&'a str>,
    /// Optional IFC date-time lexical update value.
    pub update_date: Option<&'a str>,
}
#[derive(Debug, Clone, Copy)]
/// Draft for ordered cost-item nesting through `IfcRelNests`.
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
pub struct ScheduleAssignmentDraft<'a> {
    /// Compressed IFC `GlobalId`; validated before staging.
    pub global_id: &'a str,
    /// Relating `IfcCostSchedule`.
    pub schedule: EntityId,
    /// Non-empty, duplicate-free related `IfcCostItem` set.
    pub items: &'a [EntityId],
}

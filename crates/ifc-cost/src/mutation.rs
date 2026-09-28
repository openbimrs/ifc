//! Transaction-staged cost authoring.
//!
//! The quantity writer (#190), the cost item and schedule writers and
//! their relationships (#202), and `assign_cost_quantities` (#203) bind the
//! model's declared release (IFC2X3, IFC4 or IFC4X3) and lay their records
//! out by attribute name, as `ifc-properties` does. IFC2X3 requires
//! `IfcRoot.OwnerHistory`: the `IfcRoot` writers that leave it unset refuse
//! an IFC2X3 model, and their `*_with_owner_history` variants take a
//! caller-supplied one. The cost value, currency and monetary-unit writers
//! bind the release too (#213), so each record has its release's arity.
//!
//! Assigning cost items to their schedule is authored here; assigning a cost
//! item to the products it prices is deliberately read-only
//! ([`crate::relation::controlled_by`]). Validating that an arbitrary target
//! is an `IfcObjectDefinition` needs the schema's inheritance tables, which
//! this crate reads only for the quantity layout.
#![deny(missing_docs)]

mod control;
mod datetime;
mod draft;
mod error;
mod quantity;
mod release;
mod validate;
mod value;

pub use control::{
    assign_schedule_items, assign_schedule_items_with_owner_history, create_cost_item,
    create_cost_item_with_owner_history, create_cost_schedule,
    create_cost_schedule_with_owner_history, nest_cost_items, nest_cost_items_with_owner_history,
};
pub use datetime::{CalendarDate, DateTimeValue, LocalTime};
pub use draft::{
    CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType, CostValueDraft,
    CostValueKind, NestingDraft, ScheduleAssignmentDraft,
};
pub use error::{CostAuthoringError, CostAuthoringResult};
pub use quantity::{assign_cost_quantities, create_quantity, QuantityDraft, QuantityKind};
pub use value::{create_cost_value, create_currency_relationship, create_monetary_unit};

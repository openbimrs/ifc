//! Transaction-staged IFC4 cost authoring.
//!
//! The quantity writer is the exception: it binds the model's declared
//! release (IFC2X3, IFC4 or IFC4X3) and lays its record out by attribute
//! name, as `ifc-properties` does (#190).
//!
//! Assigning cost items to their schedule is authored here; assigning a cost
//! item to the products it prices is deliberately read-only
//! ([`crate::relation::controlled_by`]). Validating that an arbitrary target
//! is an `IfcObjectDefinition` needs the schema's inheritance tables, and this
//! crate depends on `ifc-model` alone.
#![deny(missing_docs)]

mod control;
mod draft;
mod error;
mod quantity;
mod release;
mod validate;
mod value;

pub use control::{assign_schedule_items, create_cost_item, create_cost_schedule, nest_cost_items};
pub use draft::{
    CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType, CostValueDraft,
    CostValueKind, NestingDraft, ScheduleAssignmentDraft,
};
pub use error::{CostAuthoringError, CostAuthoringResult};
pub use quantity::{assign_cost_quantities, create_quantity, QuantityDraft, QuantityKind};
pub use value::{create_cost_value, create_currency_relationship, create_monetary_unit};

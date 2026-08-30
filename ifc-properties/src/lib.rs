//! `ifc-properties` -- Property sets, quantities and units -- the non-geometric payload most
//!
//! consumers actually want.
//!
//! `references/ifc-spec/` ships the official property set definitions as XML:
//! 317 for IFC2x3 and 420 for IFC4. That is a machine-readable catalogue, so
//! standard Psets are data here rather than hand-written tables.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `pset` | `IfcPropertySet` and single/enumerated/list/table properties |
//! | `quantity` | `IfcElementQuantity`: length, area, volume, weight, count |
//! | `template` | `IfcPropertySetTemplate` and property templates |
//! | `standard` | The official Pset catalogue from the shipped XML definitions |
//! | `unit` | Unit assignment, prefixes and conversion-based units |
//! | `value` | `IfcValue` measure types and their interpretation |
//! | `query` | Lookup helpers: property by name, pset by element |
//! | `error` | Why a property lookup failed |
//!
//! # Status
//!
//! Partial. `value`, `pset`, `quantity`, `unit`, `template` and `query` are
//! implemented. `PROP-EDIT` -- transactional authoring of quantities -- is
//! blocked on `ifc-model`'s `MODEL-MUT`, and `standard` (the shipped Pset
//! catalogue) is not yet read. See `../PLAN.md`.
//!
//! # What this crate will not do
//!
//! It never computes a shape measurement. An `IfcQuantityArea` is what the
//! authoring tool asserted, and it may disagree with the geometry. Callers
//! that want a check compute the value with a geometry service and pass it to
//! [`compare`], which reports agreement rather than inventing it.

mod error;
mod pset;
mod quantity;
mod query;
mod standard;
mod template;
mod unit;
mod value;

pub use error::{PropertyAnomaly, PropertyError};
pub use pset::{
    property, property_set, property_sets_by_object, AttachedSets, Attachment, Property,
    PropertySet, PropertyValue,
};
pub use quantity::{
    add_quantity_to_set, compare, create_quantity, set_description, set_name, set_quantity_value,
    Comparison, ComputedQuantity, Tolerance,
};
pub use quantity::{quantity_set, quantity_sets, stated_unit, Quantity, QuantityKind, QuantitySet};
pub use query::{
    properties_of, property_value, resolved_properties, ResolvedProperties, ResolvedSet, Source,
};
pub use template::{
    property_set_template, property_set_templates, property_template, template_of_set,
    PropertySetTemplate, PropertyTemplate,
};
pub use unit::{prefix_exponent, project_unit_for, project_units, unit, unit_type, UnitKind};
pub use value::{MeasureValue, Scalar};

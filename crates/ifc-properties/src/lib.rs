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
//! | `template` | `IfcPropertySetTemplate`, property templates, and sets checked against them |
//! | `standard` | The official Pset catalogue from the shipped XML definitions |
//! | `unit` | Unit assignment, prefixes and conversion-based units |
//! | `value` | `IfcValue` measure types and their interpretation |
//! | `query` | Lookup helpers: property by name, pset by element |
//! | `nesting` | Bounded, cycle-aware traversal of complex properties and quantities |
//! | `error` | Why a property lookup failed |
//!
//! # Status
//!
//! Implemented: `value`, `pset`, `quantity` (including transactional quantity
//! edits), `unit`, `template`, `query` and the release-bound `exact` lookups.
//! `standard` is a reserved scaffold; the shipped Pset/Qto catalogue is
//! `ifc-template-catalog`. Open work is tracked in GitHub issues.
//!
//! # What this crate will not do
//!
//! It never computes a shape measurement. An `IfcQuantityArea` is what the
//! authoring tool asserted, and it may disagree with the geometry. Callers
//! that want a check compute the value with a geometry service and pass it to
//! [`compare`], which reports agreement rather than inventing it.

mod error;
mod exact;
mod nesting;
mod pset;
mod quantity;
mod query;
mod standard;
mod template;
mod unit;
mod value;

pub use error::{PropertyAnomaly, PropertyError, PropertyResult, TemplateError};
pub use exact::{
    exact_predefined_sets, exact_properties, exact_properties_where, exact_property, exact_schema,
    exact_unit, ExactBoundedValue, ExactEntityRef, ExactEnumeratedValue, ExactEnumeration,
    ExactLogical, ExactPredefinedSet, ExactProperty, ExactPropertyEntry, ExactPropertyError,
    ExactReferenceValue, ExactResolution, ExactSource, ExactTableRow, ExactTableValue,
    ExactTypedValue, ExactUnit, ExactUnitError, ExactValue,
};
/// The IFC release an exact resolution binds to (re-exported from `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use pset::{
    add_complex_property, add_complex_property_template, add_door_lining_properties,
    add_door_panel_properties, add_element_quantity, add_permeable_covering_properties,
    add_physical_complex_quantity, add_profile_properties, add_property_bounded_value,
    add_property_dependency_relationship, add_property_enumerated_value, add_property_enumeration,
    add_property_list_value, add_property_reference_value, add_property_set,
    add_property_set_template, add_property_single_value, add_property_table_value,
    add_reinforcement_bar_properties, add_reinforcement_definition_properties,
    add_section_properties, add_section_reinforcement_properties, add_window_lining_properties,
    add_window_panel_properties, attach_property_set, attach_template, attach_type, bounded_slot,
    complex_quantity_slot, complex_slot, defines_by_template_slot, defines_by_type_slot,
    defines_slot, element_quantity_slot, enumerated_slot, list_slot, property, property_checked,
    property_set, property_set_checked, property_sets_by_object, pset_slot, pset_template_slot,
    reference_slot, single_value_slot, table_slot, AttachedSets, Attachment, DoorLiningDraft,
    Property, PropertySet, PropertyValue, ReinforcementBarDraft, SectionReinforcementDraft,
    TableValueDraft, WindowLiningDraft,
};
pub use quantity::{
    add_quantity_to_set, compare, create_quantity, create_quantity_with, set_description, set_name,
    set_quantity_value, Comparison, ComputedQuantity, QuantityExtras, Tolerance,
};
pub use quantity::{
    quantity_set, quantity_sets, stated_unit, Quantity, QuantityKind, QuantitySet, UnresolvedValue,
};
pub use query::{
    properties_of, property_value, resolved_properties, ResolvedProperties, ResolvedSet, Source,
};
pub use template::{
    property_set_template, property_set_template_checked, property_set_templates,
    property_template, property_template_checked, template_deviations, template_of_set,
    MeasureRole, PropertySetTemplate, PropertyTemplate, PropertyTemplateKind, TemplateFinding,
    TemplateReport, UndecidedReason,
};
pub use unit::{
    add_context_dependent_unit, add_conversion_based_unit, add_conversion_based_unit_with_offset,
    add_derived_unit, add_derived_unit_element, add_dimensional_exponents, add_measure_with_unit,
    add_monetary_unit, add_si_unit, assign_units, prefix_exponent, project_unit_for, project_units,
    unit, unit_type, ConversionBasedUnitDraft, MonetaryUnitDraft, SiUnitDraft, UnitKind,
};
pub use value::{MeasureValue, Scalar};

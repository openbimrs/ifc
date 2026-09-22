//! `IfcPropertySet` and single/enumerated/list/table properties.
//!

//! ## Internal split
//!
//! - `set.rs`: IfcPropertySet and relationships.
//! - `scalar.rs`: single/bounded/list/enumerated values.
//! - `table.rs`: table values and interpolation metadata.
//! - `reference.rs`: object/reference properties.
//! - `complex.rs`: nested complex properties.

mod complex;
mod reference;
pub(crate) mod scalar;
mod set;
mod table;

mod aggregate;
mod authoring;
mod predefined;
mod reinforcement;
mod template_authoring;

pub use authoring::{
    add_complex_property, add_element_quantity, add_physical_complex_quantity,
    add_property_bounded_value, add_property_enumerated_value, add_property_list_value,
    add_property_reference_value, add_property_set, add_property_single_value,
    add_property_table_value, attach_property_set, bounded_slot, complex_quantity_slot,
    complex_slot, defines_slot, element_quantity_slot, enumerated_slot, list_slot, pset_slot,
    reference_slot, single_value_slot, table_slot, TableValueDraft,
};
pub use predefined::{
    add_complex_property_template, add_door_lining_properties, add_door_panel_properties,
    add_permeable_covering_properties, add_property_dependency_relationship,
    add_property_enumeration, add_window_lining_properties, add_window_panel_properties,
    DoorLiningDraft, WindowLiningDraft,
};
pub use reinforcement::{
    add_profile_properties, add_reinforcement_bar_properties,
    add_reinforcement_definition_properties, add_section_properties,
    add_section_reinforcement_properties, ReinforcementBarDraft, SectionReinforcementDraft,
};
pub use scalar::{property, Property, PropertyValue};
pub use set::{property_set, property_sets_by_object, AttachedSets, Attachment, PropertySet};
pub use template_authoring::{
    add_property_set_template, attach_template, attach_type, defines_by_template_slot,
    defines_by_type_slot, pset_template_slot,
};

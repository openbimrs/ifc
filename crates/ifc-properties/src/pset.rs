//! `IfcPropertySet` and single/enumerated/list/table properties.
//!

//! ## Internal split
//!
//! - `set.rs`: IfcPropertySet and relationships.
//! - `scalar.rs`: single/bounded/list/enumerated values.
//! - `complex.rs`: nested complex properties.
//! - `root_authoring.rs`: release-bound `IfcRoot` writers and their
//!   `*_with_owner_history` variants.
//! - `predefined.rs`, `lining.rs`, `reinforcement.rs`: predefined property
//!   sets; `template_authoring.rs`: templates. `owned.rs` lays out their
//!   `*_with_owner_history` variants in the model's release (#202).

mod complex;
pub(crate) mod scalar;
mod set;

mod authoring;
mod lining;
mod owned;
mod predefined;
mod reinforcement;
mod root_authoring;
mod template_authoring;

pub use authoring::{
    add_complex_property, add_element_quantity, add_physical_complex_quantity,
    add_property_bounded_value, add_property_enumerated_value, add_property_list_value,
    add_property_reference_value, add_property_set, add_property_single_value,
    add_property_table_value, attach_property_set, bounded_slot, complex_quantity_slot,
    complex_slot, defines_slot, element_quantity_slot, enumerated_slot, list_slot, pset_slot,
    reference_slot, single_value_slot, table_slot, TableValueDraft,
};
pub use lining::{
    add_door_lining_properties, add_door_lining_properties_with_owner_history,
    add_window_lining_properties, add_window_lining_properties_with_owner_history, DoorLiningDraft,
    WindowLiningDraft,
};
pub use predefined::{
    add_door_panel_properties, add_door_panel_properties_with_owner_history,
    add_permeable_covering_properties, add_permeable_covering_properties_with_owner_history,
    add_property_dependency_relationship, add_property_enumeration, add_window_panel_properties,
    add_window_panel_properties_with_owner_history,
};
pub use reinforcement::{
    add_profile_properties, add_reinforcement_bar_properties,
    add_reinforcement_definition_properties,
    add_reinforcement_definition_properties_with_owner_history, add_section_properties,
    add_section_reinforcement_properties, ReinforcementBarDraft, SectionReinforcementDraft,
};
pub use root_authoring::{
    add_element_quantity_with_owner_history, add_property_set_with_owner_history,
    attach_property_set_with_owner_history, attach_type_with_owner_history,
};
pub use scalar::{property, property_checked, Property, PropertyValue};
pub use set::{
    property_set, property_set_checked, property_sets_by_object, AttachedSets, Attachment,
    PropertySet,
};
pub use template_authoring::{
    add_complex_property_template, add_complex_property_template_with_owner_history,
    add_property_set_template, add_property_set_template_with_owner_history, attach_template,
    attach_template_with_owner_history, attach_type, defines_by_template_slot,
    defines_by_type_slot, pset_template_slot,
};

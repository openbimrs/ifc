//! `IfcPropertySetTemplate` and property templates.
//!

//! ## Internal split
//!
//! - `property_set.rs`: set templates.
//! - `property.rs`: property templates.

mod property;
mod property_set;

mod relationship;

pub use property_set::{
    property_set_template, property_set_templates, property_template, template_of_set,
    PropertySetTemplate, PropertyTemplate,
};

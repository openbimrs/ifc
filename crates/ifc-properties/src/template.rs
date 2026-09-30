//! `IfcPropertySetTemplate` and property templates.
//!

//! ## Internal split
//!
//! - `layout.rs`: attributes read by name from the bound release's table.
//! - `property_set.rs`: set templates and `IfcRelDefinesByTemplate`.
//! - `property.rs`: simple and complex property templates, with bounded,
//!   cycle-aware traversal of nested templates.
//! - `check.rs`: property sets compared with their in-file templates.

mod check;
mod layout;
mod property;
mod property_set;

pub use check::{
    template_deviations, MeasureRole, TemplateFinding, TemplateReport, UndecidedReason,
};
pub use property::{
    property_template, property_template_checked, PropertyTemplate, PropertyTemplateKind,
};
pub use property_set::{
    property_set_template, property_set_template_checked, property_set_templates, template_of_set,
    PropertySetTemplate,
};

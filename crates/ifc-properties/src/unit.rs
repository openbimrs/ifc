//! Unit assignment, prefixes and conversion-based units.
//!
//! `IfcUnitAssignment` sets the model's units; derived and conversion-based
//! units (imperial, US survey feet) must be resolved before a value means
//! anything.

//! ## Internal split
//!
//! - `assignment.rs`: project unit context.
//! - `si.rs`: SI prefixes, per-release dimension tables, base-unit scales.
//! - `conversion.rs`: conversion-based units.
//! - `derived.rs`: derived dimensions/elements.

mod assignment;
mod authoring;
mod conversion;
mod derived;
pub(crate) mod si;

mod monetary;

pub use assignment::{prefix_exponent, project_unit_for, project_units, unit, unit_type, UnitKind};
pub use authoring::{
    add_context_dependent_unit, add_conversion_based_unit, add_conversion_based_unit_with_offset,
    add_derived_unit, add_derived_unit_element, add_dimensional_exponents, add_measure_with_unit,
    add_monetary_unit, add_si_unit, assign_units, ConversionBasedUnitDraft, MonetaryUnitDraft,
    SiUnitDraft,
};

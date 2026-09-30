//! Unit assignment, prefixes and conversion-based units.
//!
//! `IfcUnitAssignment` sets the model's units; derived and conversion-based
//! units (imperial, US survey feet) must be resolved before a value means
//! anything.
//!
//! This module serves the permissive views. Exact, release-bound unit
//! traversal (`exact_unit`) lives in `crate::exact::unit`, because it needs
//! the record-arity and domain checks there; `conversion.rs` and
//! `derived.rs` own conversion-based and derived units for the permissive
//! views only.
//!
//! ## Internal split
//!
//! - `assignment.rs`: project unit context.
//! - `si.rs`: SI prefixes, per-release dimension tables, base-unit scales.

mod assignment;
mod authoring;
pub(crate) mod si;

pub use assignment::{prefix_exponent, project_unit_for, project_units, unit, unit_type, UnitKind};
// Deprecated (#232), still exported so 0.6 callers keep compiling.
#[allow(deprecated)]
pub use authoring::add_monetary_unit;
pub use authoring::{
    add_context_dependent_unit, add_conversion_based_unit, add_conversion_based_unit_with_offset,
    add_derived_unit, add_derived_unit_element, add_dimensional_exponents, add_measure_with_unit,
    add_si_unit, assign_units, create_monetary_unit, ConversionBasedUnitDraft, MonetaryUnitDraft,
    SiUnitDraft,
};

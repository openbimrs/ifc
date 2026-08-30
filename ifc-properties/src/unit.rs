//! Unit assignment, prefixes and conversion-based units.
//!
//! `IfcUnitAssignment` sets the model's units; derived and conversion-based
//! units (imperial, US survey feet) must be resolved before a value means
//! anything.

//! ## Internal split
//!
//! - `assignment.rs`: project unit context.
//! - `si.rs`: SI prefixes/dimensions.
//! - `conversion.rs`: conversion-based units.
//! - `derived.rs`: derived dimensions/elements.

mod assignment;
mod conversion;
mod derived;
mod si;

mod monetary;

pub use assignment::{prefix_exponent, project_unit_for, project_units, unit, unit_type, UnitKind};

//! Lookup helpers: property by name, pset by element.
//!

//! ## Internal split
//!
//! - `assignment.rs`: object/type set assignment.
//! - `quantity.rs`: the same resolution for quantity sets.

mod assignment;
mod quantity;

pub use assignment::{
    properties_of, property_value, resolved_properties, ResolvedProperties, ResolvedSet, Source,
};
pub use quantity::{
    quantities_of, quantity_value, resolved_quantities, ResolvedQuantities, ResolvedQuantitySet,
};

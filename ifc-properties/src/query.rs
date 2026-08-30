//! Lookup helpers: property by name, pset by element.
//!

//! ## Internal split
//!
//! - `assignment.rs`: object/type set assignment.

mod assignment;

pub use assignment::{
    properties_of, property_value, resolved_properties, ResolvedProperties, ResolvedSet, Source,
};

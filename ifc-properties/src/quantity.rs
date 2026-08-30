//! `IfcElementQuantity`: length, area, volume, weight, count.
//!
//! Quantities authored in the file, as distinct from quantities derived from
//! geometry -- the two disagree often enough that mixing them silently is a bug.

//! ## Internal split
//!
//! - `set.rs`: IfcElementQuantity.
//! - `simple.rs`: length/area/volume/count/time/weight.
//! - `complex.rs`: nested physical complex quantities.
//! - `edit.rs`: transactional authored quantity updates.
//! - `validation.rs`: units/dimensions/formula consistency.

mod complex;
mod edit;
mod set;
mod simple;
mod validation;

pub use set::{quantity_set, quantity_sets, stated_unit, Quantity, QuantityKind, QuantitySet};
pub use validation::{compare, Comparison, ComputedQuantity, Tolerance};

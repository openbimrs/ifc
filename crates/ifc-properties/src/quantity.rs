//! `IfcElementQuantity`: length, area, volume, weight, count.
//!
//! Quantities authored in the file, as distinct from quantities derived from
//! geometry -- the two disagree often enough that mixing them silently is a bug.

//! ## Internal split
//!
//! - `set.rs`: IfcElementQuantity.
//! - `simple.rs`: length/area/volume/count/time/weight, with or without a
//!   readable value.
//! - `complex.rs`: nested physical complex quantities.
//! - `edit.rs`: transactional authored quantity updates.
//! - `release.rs`: the declared release's layout those updates write.
//! - `validation.rs`: units/dimensions/formula consistency.

mod complex;
mod edit;
pub(crate) mod release;
mod set;
mod simple;
mod validation;

pub use edit::{
    add_quantity_to_set, create_quantity, create_quantity_with, set_description, set_name,
    set_quantity_value, QuantityExtras,
};
pub use set::{quantity_set, quantity_sets, stated_unit, Quantity, QuantityKind, QuantitySet};
pub use simple::UnresolvedValue;
pub use validation::{compare, Comparison, ComputedQuantity, Tolerance};

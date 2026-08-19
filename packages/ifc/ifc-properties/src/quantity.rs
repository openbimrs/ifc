//! `IfcElementQuantity`: length, area, volume, weight, count.
//!
//! Quantities authored in the file, as distinct from quantities derived from
//! geometry -- the two disagree often enough that mixing them silently is a bug.
//!
//! Implementation is tracked in `../PLAN.md`.

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

//! `IfcMapConversion`: local engineering to map coordinates.
//!
//! Eastings, northings, orthogonal height, X-axis abscissa/ordinate and scale.
//!
//! # Pitfall
//!
//! Site coordinates routinely exceed `f32` precision -- another reason the
//! kernel stores `f64` (see `docs/adr/0001`).
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `map.rs`: IfcMapConversion parameters.
//! - `rigid.rs`: rigid coordinate operations where schema permits.

mod map;
mod rigid;

mod validation;

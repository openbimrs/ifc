//! Attribute values match their declared EXPRESS types.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `entity.rs`: entity/subtype compatibility.
//! - `select.rs`: SELECT membership.
//! - `defined.rs`: defined-type chains.
//! - `enumeration.rs`: enumeration and logical forms.
//! - `scalar.rs`: scalar value forms.

mod defined;
mod entity;
mod enumeration;
mod scalar;
mod select;

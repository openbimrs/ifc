//! Value checking against declared EXPRESS types.
//!
//! ## Internal split
//!
//! - `declared.rs`: resolve a declared type token to the value shape it admits.
//! - `form.rs`: whether a declared type's values are written typed or bare.
//! - `derived.rs`: which inherited slots the schema derives for an entity.

mod declared;
mod derived;
mod form;

pub(crate) use declared::{aggregate_element, describe_value, judge_value, Verdict};
pub(crate) use derived::is_derived_slot;

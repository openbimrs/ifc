//! Value checking against declared EXPRESS types.
//!
//! ## Internal split
//!
//! - `declared.rs`: resolve a declared type token to the value shape it admits.
//! - `derived.rs`: which inherited slots the schema derives for an entity.

mod declared;
mod derived;

pub(crate) use declared::{aggregate_element, describe_value, value_matches};
pub(crate) use derived::is_derived_slot;

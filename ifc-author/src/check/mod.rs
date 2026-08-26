//! Value checking against declared EXPRESS types.
//!
//! ## Internal split
//!
//! - `declared.rs`: resolve a declared type token to the value shape it admits.

mod declared;

pub(crate) use declared::{describe_value, value_matches};

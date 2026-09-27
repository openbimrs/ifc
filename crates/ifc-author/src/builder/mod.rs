//! Constructing entities by attribute name.
//!
//! ## Internal split
//!
//! - `entity.rs`: the named-attribute builder and its slot resolution.

mod entity;

pub(crate) use entity::check_value;
pub use entity::EntityBuilder;

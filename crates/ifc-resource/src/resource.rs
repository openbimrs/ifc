//! The resource hierarchy and common attributes.
//!
//! ## Internal split
//!
//! - `base.rs`: construction resource base.
//! - `type.rs`: resource types.
//! - `nesting.rs`: resource composition.

mod base;
mod nesting;
mod r#type;

pub use base::{ConstructionResource, ResourceKind};
pub use r#type::{ConstructionResourceType, ResourceTypeKind};

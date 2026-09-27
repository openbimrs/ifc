//! Flow direction and segment/fitting/terminal roles.
//!
//!

//! ## Internal split
//!
//! - `direction.rs`: flow direction/select semantics.
//! - `role.rs`: source/sink role.

mod direction;
mod role;

pub use direction::FlowDirection;
pub use role::{role_inconsistencies, ElementRole, RoleInconsistency};

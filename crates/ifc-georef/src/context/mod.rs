//! Coordinate-operation contexts: the operation's source, and composing a
//! project frame onto a resolved operation.

//! ## Internal split
//!
//! - `chain.rs`: project-frame to map-frame composition contract; only
//!   with the `transform` feature, since its input and output are
//!   `axiolid_core::Transform3`.
//! - `source.rs`: `SourceCRS` validation and the `HasCoordinateOperation`
//!   inverse.

#[cfg(feature = "transform")]
mod chain;
mod source;

#[cfg(feature = "transform")]
pub use chain::compose_project_frame;
pub use source::{coordinate_operation_for, resolve_operation_source, OperationSource};

pub(crate) use source::operation_source;

//! Coordinate-operation contexts: the operation's source, and composing a
//! project frame onto a resolved operation.

//! ## Internal split
//!
//! - `chain.rs`: project-frame to map-frame composition contract.
//! - `source.rs`: `SourceCRS` validation and the `HasCoordinateOperation`
//!   inverse.

mod chain;
mod source;

pub use chain::compose_project_frame;
pub use source::{coordinate_operation_for, resolve_operation_source, OperationSource};

pub(crate) use source::operation_source;

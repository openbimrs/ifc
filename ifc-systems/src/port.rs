//! `IfcDistributionPort` and port assignment to elements.
//!
//!

//! ## Internal split
//!
//! - `definition.rs`: IfcPort/DistributionPort.
//! - `assignment.rs`: port nesting/attachment.

mod assignment;
mod definition;

pub use definition::{ports, Attachment, FlowDirection, Port};

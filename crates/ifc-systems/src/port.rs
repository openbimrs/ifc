//! `IfcDistributionPort` and port assignment to elements.
//!
//!

//! ## Internal split
//!
//! - `definition.rs`: IfcPort/DistributionPort.

pub(crate) mod definition;

pub use definition::{ports, Attachment, Port};

//! `IfcRelConnectsPorts` and network traversal.
//!
//! The graph walk that answers 'what is downstream of this valve'. Cycles are
//! legal here (ring mains), so traversal must handle them by design.
//!
//! Connectivity comes only from IFC relationships (`IfcRelConnectsPorts`,
//! port nesting and attachment), never from geometric proximity: two ducts
//! that touch in space but share no relationship are not connected here.
//!
//! ## Internal split
//!
//! - `relation.rs`: port/element connections.
//! - `traversal.rs`: bounded traversal.

pub(crate) mod relation;
mod traversal;

pub use relation::{Connection, ConnectionGraph, NetworkGraph};
pub use traversal::{Direction, FlowNetwork, FlowQuery};

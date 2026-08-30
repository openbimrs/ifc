//! `ifc-systems` -- Distribution systems: ports, connectivity and system grouping.
//!
//!
//! 23 entities in IFC4. Turns a bag of pipes and ducts into a connected network
//! that can be traced -- the basis of any MEP analysis.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `system` | `IfcSystem`, `IfcDistributionSystem` and grouping |
//! | `port` | `IfcDistributionPort` and port assignment to elements |
//! | `connectivity` | `IfcRelConnectsPorts` and network traversal |
//! | `flow` | Flow direction and segment/fitting/terminal roles |
//! | `error` | Why a system query failed |
//!
//! # Status
//!
//! Partial. Implemented: SYS-ROOT (systems, subtype-aware discovery and
//! membership), SYS-PORT (ports and both element-attachment forms) and
//! SYS-CONN (the undirected connection graph with cycle-safe traversal).
//! `flow`, `zone` and `assignment` are reserved with intent, not implemented
//! -- see `../PLAN.md` for the stages that fill them.

mod connectivity;
mod error;
mod flow;
mod port;
mod system;

pub use connectivity::{Connection, ConnectionGraph, NetworkGraph};
pub use error::SystemAnomaly;
pub use port::{ports, Attachment, FlowDirection, Port};
pub use system::{systems, System};

mod assignment;
mod zone;

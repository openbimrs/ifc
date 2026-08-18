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
//! | [`system`] | `IfcSystem`, `IfcDistributionSystem` and grouping |
//! | [`port`] | `IfcDistributionPort` and port assignment to elements |
//! | [`connectivity`] | `IfcRelConnectsPorts` and network traversal |
//! | [`flow`] | Flow direction and segment/fitting/terminal roles |
//! | [`error`] | Why a system query failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `docs/ROADMAP.md` for the stage that fills them.

pub mod connectivity;
pub mod error;
pub mod flow;
pub mod port;
pub mod system;

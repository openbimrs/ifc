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
//! Implemented: systems with subtype-aware discovery and membership, ports
//! and both element-attachment forms, the undirected connection graph with
//! cycle-safe traversal, flow roles and direction, and zones. `assignment`
//! (services-building relationships) is a reserved scaffold.

pub mod authoring;
mod connectivity;
mod error;
mod flow;
mod port;
mod release;
mod system;

pub use authoring::{
    assign_to_group, connect_port_to_element, connect_ports, contain_in_spatial_structure,
    create_classified_system, create_group, create_port, create_system, nest_ports,
    reference_in_spatial_structure, ClassifiedSystemDraft, SystemAuthoringError,
    SystemAuthoringResult, SystemKind,
};
pub use connectivity::{
    Connection, ConnectionGraph, Direction, FlowNetwork, FlowQuery, NetworkGraph,
};
pub use error::{NotInSchema, SchemaGap, SchemaResolutionError, SystemAnomaly};
pub use flow::{role_inconsistencies, ElementRole, FlowDirection, RoleInconsistency};
/// The IFC release a read binds to (re-exported from `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use port::{ports, Attachment, Port};
pub use release::schema_of;
pub use system::{systems, System};
pub use zone::{long_name_of, spatial_placements, zones, SpatialPlacement, Zone};

mod assignment;
pub(crate) mod zone;

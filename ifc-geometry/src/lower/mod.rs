//! IFC-to-neutral-geometry lowering entry points.

pub mod boolean;
pub mod brep;
pub mod context;
pub mod dispatch;
pub mod mapped;
pub mod profile;
pub mod session;
pub mod swept;
pub mod tolerance;

use axiolid_model::{GeometryGraph, NodeId};

pub use boolean::lower_boolean_result_node;
pub use brep::lower_faceted_brep_node;
pub use context::{
    geometric_products, lower_product_items, product_world_transform, select_shape_representation,
};
pub use dispatch::lower_representation_item;
pub use mapped::{lower_mapped_item_node, lower_representation};
pub use profile::{lower_profile, lower_profile_node};
pub use provenance::ProvenanceMap;
pub use session::{LoweringSession, SessionLimits};
pub use swept::{
    lower_extruded_area_solid, lower_extruded_area_solid_node, lower_revolved_area_solid,
    lower_revolved_area_solid_node,
};
pub use tolerance::Tolerance;

/// One lowered root and the immutable DAG that owns all of its dependencies.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredGeometry {
    /// Format-neutral exact geometry graph.
    pub graph: GeometryGraph,
    /// Root node for this source representation item.
    pub root: NodeId,
    /// IFC source entity for each attributed graph node.
    pub provenance: ProvenanceMap,
}

mod curve;
mod placement;
mod provenance;
mod solid;
mod surface;
mod tessellated;

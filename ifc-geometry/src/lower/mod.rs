//! IFC-to-neutral-geometry lowering entry points.

pub mod bbox;
pub mod boolean;
pub mod brep;
pub mod collection;
pub mod context;
pub mod csg;
pub mod curve;
pub mod dispatch;
pub mod halfspace;
pub mod mapped;
pub mod profile;
pub mod session;
pub mod surface;
pub mod swept;
pub mod tessellated;

use axiolid_model::{GeometryGraph, NodeId};

pub use crate::input::representation::RepresentationPurpose;
pub use bbox::lower_bounding_box_node;
pub use boolean::lower_boolean_result_node;
pub use brep::{lower_faceted_brep_node, lower_shell_node};
pub use collection::lower_collection_node;
pub use context::{
    geometric_products, lower_product_items, lower_product_representation, product_world_transform,
    select_shape_representation,
};
pub use csg::{lower_csg_primitive_node, lower_csg_solid_node, lower_swept_disk_node};
pub use curve::lower_curve_node;
pub use dispatch::lower_representation_item;
pub use halfspace::lower_half_space_node;
pub use mapped::{lower_mapped_item_node, lower_representation};
pub use profile::{lower_open_profile_node, lower_profile, lower_profile_node};
pub use provenance::ProvenanceMap;
pub use session::{LoweringSession, SessionLimits};
pub use surface::{lower_linear_extrusion, lower_plane, lower_surface_node};
pub use swept::{
    lower_extruded_area_solid, lower_extruded_area_solid_node, lower_revolved_area_solid,
    lower_revolved_area_solid_node,
};
pub use tessellated::{lower_polygonal_face_set_node, lower_triangulated_face_set_node};

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

mod placement;
mod provenance;
mod solid;

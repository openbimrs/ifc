//! IFC-to-neutral-geometry lowering entry points.

pub mod profile;
pub mod swept;
pub mod tolerance;

use geom_model::{GeometryGraph, NodeId};

pub use profile::lower_profile;
pub use swept::{lower_extruded_area_solid, lower_revolved_area_solid};
pub use tolerance::Tolerance;

/// One lowered root and the immutable DAG that owns all of its dependencies.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweredGeometry {
    /// Format-neutral exact geometry graph.
    pub graph: GeometryGraph,
    /// Root node for this source representation item.
    pub root: NodeId,
}

mod boolean;
mod brep;
mod context;
mod curve;
mod dispatch;
mod mapped;
mod placement;
mod provenance;
mod session;
mod solid;
mod surface;
mod tessellated;

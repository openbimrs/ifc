//! Feature-gated facade for Nehirde geometry.
//!
//! The default build is intentionally small: core values, meshes, and the
//! portable CPU backend shell. Exact curves/surfaces/topology, algorithms,
//! parallel scheduling, and GPU adapters are opt-in. Leaf crates remain public
//! for consumers that want an even narrower dependency graph.

/// Always-available scalar, transform, and bounds vocabulary.
pub mod core {
    pub use geom_core::*;
}

pub use geom_core::{Aabb, Point2, Point3, Scalar, Tolerance, Transform3, Vec2, Vec3};

#[cfg(feature = "mesh")]
pub mod mesh {
    pub use geom_mesh::*;
}

#[cfg(feature = "profiles")]
pub mod profile {
    pub use geom_profile::*;
}

#[cfg(feature = "curves")]
pub mod curve {
    pub use geom_curve::*;
}

#[cfg(feature = "surfaces")]
pub mod surface {
    pub use geom_surface::*;
}

#[cfg(feature = "topology")]
pub mod topology {
    pub use geom_topology::*;
}

#[cfg(feature = "model")]
pub mod model {
    pub use geom_model::*;
}

#[cfg(feature = "primitives")]
pub mod primitive {
    pub use geom_primitive::*;
}

#[cfg(feature = "sweeps")]
pub mod sweep {
    pub use geom_sweep::*;
}

#[cfg(feature = "tessellation")]
pub mod tessellation {
    pub use geom_tessellate::*;
}

#[cfg(feature = "spatial")]
pub mod spatial {
    pub use geom_spatial::*;
}

#[cfg(feature = "measure")]
pub mod measure {
    pub use geom_measure::*;
}

#[cfg(feature = "heal")]
pub mod heal {
    pub use geom_heal::*;
}

#[cfg(feature = "kernel")]
pub mod kernel {
    pub use geom_kernel::*;
}

#[cfg(feature = "cpu")]
pub mod cpu {
    pub use geom_backend_cpu::*;
}

#[cfg(feature = "gpu")]
pub mod gpu {
    pub use geom_backend_gpu::*;
}

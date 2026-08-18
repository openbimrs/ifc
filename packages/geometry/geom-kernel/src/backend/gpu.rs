//! Optional GPU offload. **Off by default.**
//!
//! # Why this is opt-in and mostly empty
//!
//! This project exists to be a lightweight alternative to the
//! IfcOpenShell+OpenCascade stack. Making every consumer compile a GPU stack to
//! read a wall would reproduce exactly the problem we are solving. So GPU
//! support lives behind the `gpu` feature, off by default, and the crate
//! compiles to almost nothing without it.
//!
//! # Where a GPU actually wins (and where it does not)
//!
//! GPU offload only pays when the work is large, regular, and data-parallel
//! enough to amortize the PCIe round trip. Realistic candidates:
//! broad-phase collision over a whole federated model, batch ray casts,
//! voxelization, and mesh simplification of very large tessellations.
//!
//! It is a poor fit for the core boolean: mesh CSG is branchy, topological, and
//! precision-sensitive, and a per-element wall-minus-two-openings cut is far too
//! small to amortize a transfer. **We do not plan a GPU boolean.**
//! [`Capabilities::gpu_threshold_triangles`] encodes the size gate so this
//! judgement is enforced by the dispatcher rather than left to a comment.
//!
//! # Status
//!
//! Scaffold only: reports unavailable, offers no operations. This crate exists
//! now so the abstraction is proven to accommodate a third backend shape — one
//! that can be absent at runtime — rather than being retrofitted later.

use crate::{Backend, Capabilities};

/// GPU backend handle.
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuBackend;

impl GpuBackend {
    /// Attempt to acquire a GPU device.
    ///
    /// Returns `None` when the `gpu` feature is disabled or no device is
    /// present. Callers must handle `None` — a GPU is never assumed.
    pub fn detect() -> Option<Self> {
        #[cfg(feature = "gpu")]
        {
            // A real implementation enumerates adapters here and returns None
            // when none is suitable.
            None
        }
        #[cfg(not(feature = "gpu"))]
        {
            None
        }
    }

    /// Capabilities. Always reports unavailable in the current scaffold.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            backend: Backend::Gpu,
            available: false,
            mesh_boolean: false,
            // Below ~100k triangles a PCIe round trip costs more than it saves.
            gpu_threshold_triangles: Some(100_000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_gpu_is_a_normal_outcome_not_an_error() {
        assert!(GpuBackend::detect().is_none());
        assert!(!GpuBackend.capabilities().available);
    }
}

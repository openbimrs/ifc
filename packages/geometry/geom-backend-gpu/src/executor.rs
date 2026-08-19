//! Open graph-compiler seam implemented by concrete GPU API crates.

use geom_kernel::{BackendDescriptor, ExecutionOptions, GeomResult};
use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, NodeId};

use crate::GpuDeviceDescriptor;

/// Concrete GPU graph compiler supplied by an API-specific crate.
///
/// A CUDA executor for a workstation, a Metal executor for Apple silicon, and
/// a WebGPU executor for integrated graphics can implement this batch contract
/// without leaking vendor types into `geom-kernel`.
pub trait GpuGraphExecutor: core::fmt::Debug + Send + Sync {
    /// Hardware/API facts.
    fn device(&self) -> &GpuDeviceDescriptor;

    /// Provider identity. The trait implementation itself proves the graph
    /// compilation capability.
    fn descriptor(&self) -> BackendDescriptor;

    /// Compile graph roots as one batch to amortize upload and synchronization.
    fn compile_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>>;
}

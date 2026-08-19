//! Open executor seam implemented by concrete GPU API crates.

use geom_kernel::{BackendDescriptor, ExecutionOptions, GeomResult};
use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, NodeId};

use crate::GpuDeviceDescriptor;

/// Concrete GPU executor supplied by a downstream API-specific crate.
///
/// A CUDA executor for an RTX workstation, a Metal executor for Apple silicon,
/// and a WebGPU executor for portable integrated graphics can all implement the
/// same batch contract without leaking vendor types into `geom-kernel`.
pub trait GpuExecutor: core::fmt::Debug + Send + Sync {
    /// Hardware/API facts.
    fn device(&self) -> &GpuDeviceDescriptor;

    /// Kernel capability descriptor. Advertise only implemented operations and
    /// only precision modes the device path actually honors.
    fn descriptor(&self) -> BackendDescriptor;

    /// Compile graph roots as one batch to amortize upload and synchronization.
    fn compile_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>>;
}

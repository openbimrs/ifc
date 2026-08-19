//! Adapter from an API-specific GPU executor to the generic kernel traits.

use geom_kernel::{
    Backend, BackendDescriptor, ExecutionOptions, GeomError, GeomResult, GeometryCompiler,
};
use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, NodeId};

use crate::{GpuDeviceDescriptor, GpuExecutor};

/// Generic ownership adapter for a concrete GPU executor.
#[derive(Debug)]
pub struct GpuBackend<E> {
    executor: E,
}

impl<E> GpuBackend<E> {
    /// Wrap one initialized concrete executor.
    pub const fn new(executor: E) -> Self {
        Self { executor }
    }

    /// Underlying device facts.
    pub fn device(&self) -> &GpuDeviceDescriptor
    where
        E: GpuExecutor,
    {
        self.executor.device()
    }

    /// Borrow the API-specific executor for advanced operations.
    pub const fn executor(&self) -> &E {
        &self.executor
    }
}

impl<E: GpuExecutor> Backend for GpuBackend<E> {
    fn descriptor(&self) -> BackendDescriptor {
        self.executor.descriptor()
    }
}

impl<E: GpuExecutor> GeometryCompiler for GpuBackend<E> {
    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let results = self.executor.compile_batch(graph, &[root], options)?;
        if results.len() != 1 {
            return Err(GeomError::InvalidInput(format!(
                "GPU executor returned {} results for one root",
                results.len()
            )));
        }
        results.into_iter().next().ok_or_else(|| {
            GeomError::InvalidInput("GPU executor returned no result for one root".to_owned())
        })
    }

    fn compile_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        self.executor.compile_batch(graph, roots, options)
    }
}

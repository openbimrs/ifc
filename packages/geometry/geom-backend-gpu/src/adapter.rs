//! Adapter from an API-specific GPU executor to graph compilation.

use geom_kernel::{
    Backend, BackendDescriptor, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
    GeometryCompiler, Operation, Precision,
};
use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, NodeId};

use crate::{GpuDeviceDescriptor, GpuGraphExecutor};

/// Graph-compiler provider backed by one initialized GPU executor.
#[derive(Debug)]
pub struct GpuCompiler<E> {
    executor: E,
}

impl<E> GpuCompiler<E> {
    /// Wrap one initialized concrete executor.
    pub const fn new(executor: E) -> Self {
        Self { executor }
    }

    /// Underlying device facts.
    pub fn device(&self) -> &GpuDeviceDescriptor
    where
        E: GpuGraphExecutor,
    {
        self.executor.device()
    }

    /// Borrow the API-specific executor for advanced operations.
    pub const fn executor(&self) -> &E {
        &self.executor
    }

    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()>
    where
        E: GpuGraphExecutor,
    {
        let device = self.executor.device();
        if options.precision() == Precision::F64 && !device.features.float64 {
            return Err(GeomError::Unsupported {
                backend: device.id,
                operation: Operation::GraphCompilation,
            });
        }
        Ok(())
    }
}

impl<E: GpuGraphExecutor> Backend for GpuCompiler<E> {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(self.executor.device().id, ExecutionTarget::Gpu)
    }
}

impl<E: GpuGraphExecutor> GeometryCompiler for GpuCompiler<E> {
    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.validate_options(options)?;
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
        self.validate_options(options)?;
        self.executor.compile_batch(graph, roots, options)
    }
}

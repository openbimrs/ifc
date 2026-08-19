//! Adapter from an API-specific GPU executor to graph compilation.

use geom_kernel::{
    Backend, BackendDescriptor, DevicePreference, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, GeometryCompiler, Operation, Precision,
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
        let compatible_device = match options.device() {
            DevicePreference::Auto | DevicePreference::Gpu => true,
            DevicePreference::Backend(required) => required == device.id,
            DevicePreference::Cpu => false,
        };
        if !compatible_device || (options.precision() == Precision::F64 && !device.features.float64)
        {
            return Err(GeomError::Unsupported {
                backend: device.id,
                operation: Operation::GraphCompilation,
            });
        }
        Ok(())
    }

    fn validate_roots(graph: &GeometryGraph, roots: &[NodeId]) -> GeomResult<()> {
        if let Some(root) = roots.iter().find(|root| graph.get(**root).is_none()) {
            return Err(GeomError::InvalidInput(format!(
                "graph compilation root {root} does not belong to the supplied graph"
            )));
        }
        Ok(())
    }

    fn validate_result_count(root_count: usize, result_count: usize) -> GeomResult<()> {
        if result_count != root_count {
            return Err(GeomError::InvalidInput(format!(
                "GPU executor returned {result_count} results for {root_count} roots"
            )));
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
        Self::validate_roots(graph, &[root])?;
        let results = self.executor.compile_batch(graph, &[root], options)?;
        Self::validate_result_count(1, results.len())?;
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
        Self::validate_roots(graph, roots)?;
        let results = self.executor.compile_batch(graph, roots, options)?;
        Self::validate_result_count(roots.len(), results.len())?;
        Ok(results)
    }
}

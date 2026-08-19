//! API-neutral GPU adapter layer.
//!
//! This crate intentionally does not choose CUDA, Metal, Vulkan, or WebGPU.
//! Concrete API crates implement [`GpuExecutor`], report precision/capability
//! truthfully, and submit batches through [`GpuBackend`]. Default geometry builds
//! therefore carry no GPU dependency or driver stack.

pub mod adapter;
pub mod device;
pub mod executor;

pub use adapter::GpuBackend;
pub use device::{GpuDeviceDescriptor, GpuFeatures};
pub use executor::GpuExecutor;

#[cfg(test)]
mod tests {
    use geom_core::{Tolerance, Vec3};
    use geom_kernel::{
        Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomResult,
        GeometryCompiler, Operation, OperationSupport, Precision,
    };
    use geom_mesh::TriMesh;
    use geom_model::{GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId};

    use super::*;

    #[derive(Debug)]
    struct FakeExecutor {
        device: GpuDeviceDescriptor,
    }

    impl GpuExecutor for FakeExecutor {
        fn device(&self) -> &GpuDeviceDescriptor {
            &self.device
        }

        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor {
                id: BackendId::new("test-gpu"),
                target: ExecutionTarget::Gpu,
                available: true,
                unavailable_reason: None,
                operations: vec![OperationSupport {
                    operation: Operation::GraphCompilation,
                    precision: vec![Precision::F64],
                    deterministic: true,
                    minimum_batch_size: 1,
                }],
            }
        }

        fn compile_batch(
            &self,
            _graph: &GeometryGraph,
            roots: &[NodeId],
            _options: &ExecutionOptions,
        ) -> GeomResult<Vec<TriMesh>> {
            Ok(roots.iter().map(|_| TriMesh::default()).collect())
        }
    }

    #[test]
    fn downstream_executor_plugs_in_without_vendor_types_in_kernel() {
        let executor = FakeExecutor {
            device: GpuDeviceDescriptor {
                name: "test".to_owned(),
                api: "mock".to_owned(),
                features: GpuFeatures {
                    float64: true,
                    subgroups: false,
                    unified_memory: true,
                    max_workgroup_size: 64,
                },
            },
        };
        let backend = GpuBackend::new(executor);
        let mut builder = GeometryGraphBuilder::new();
        let root = builder.push(GeometryNode::Point3(Vec3::ZERO)).unwrap();
        let graph = builder.finish(vec![root]).unwrap();
        let options = ExecutionOptions::new(Tolerance::METRE);
        assert_eq!(
            backend.compile(&graph, root, &options).unwrap(),
            TriMesh::default()
        );
        assert_eq!(backend.descriptor().id, BackendId::new("test-gpu"));
    }
}

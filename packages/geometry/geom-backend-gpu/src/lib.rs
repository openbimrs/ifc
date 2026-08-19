#![forbid(unsafe_code)]

//! API-neutral GPU operation adapters.
//!
//! This crate intentionally chooses no CUDA, Metal, Vulkan, or WebGPU library.
//! Concrete API crates implement a narrow operation executor and submit batches;
//! default geometry builds carry no GPU dependency or driver stack.

pub mod adapter;
pub mod device;
pub mod executor;

pub use adapter::GpuCompiler;
pub use device::{GpuDeviceDescriptor, GpuFeatures};
pub use executor::GpuGraphExecutor;

#[cfg(test)]
mod tests {
    use geom_core::{Tolerance, Vec3};
    use geom_kernel::{
        Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomResult,
        GeometryCompiler,
    };
    use geom_mesh::TriMesh;
    use geom_model::{GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId};

    use super::*;

    #[derive(Debug)]
    struct FakeExecutor {
        device: GpuDeviceDescriptor,
    }

    impl GpuGraphExecutor for FakeExecutor {
        fn device(&self) -> &GpuDeviceDescriptor {
            &self.device
        }

        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor {
                id: BackendId::new("test-gpu-compiler"),
                target: ExecutionTarget::Gpu,
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
    fn downstream_executor_proves_its_capability_by_implementing_the_trait() {
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
        let compiler = GpuCompiler::new(executor);
        let mut builder = GeometryGraphBuilder::new();
        let root = builder
            .push(GeometryNode::Point3(Vec3::ZERO))
            .expect("root");
        let graph = builder.finish(vec![root]).expect("graph");
        let options = ExecutionOptions::new(Tolerance::METRE);
        assert_eq!(
            compiler.compile(&graph, root, &options).expect("compile"),
            TriMesh::default()
        );
        assert_eq!(
            compiler.descriptor().id,
            BackendId::new("test-gpu-compiler")
        );
    }
}

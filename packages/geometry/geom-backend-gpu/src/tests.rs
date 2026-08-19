use geom_core::{Tolerance, Vec3};
use geom_kernel::{
    Backend, BackendId, ExecutionOptions, GeomError, GeomResult, GeometryCompiler, Operation,
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
            id: BackendId::new("test-gpu-compiler"),
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

    let mut f32_device = compiler.device().clone();
    f32_device.id = BackendId::new("f32-only");
    f32_device.features.float64 = false;
    let f32_only = GpuCompiler::new(FakeExecutor { device: f32_device });
    assert!(matches!(
        f32_only.compile(&graph, root, &options),
        Err(GeomError::Unsupported {
            backend,
            operation: Operation::GraphCompilation,
        }) if backend == BackendId::new("f32-only")
    ));
}

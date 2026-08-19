use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use geom_core::{Tolerance, Vec3};
use geom_kernel::{
    Backend, BackendId, DevicePreference, ExecutionOptions, GeomError, GeomResult,
    GeometryCompiler, Operation,
};
use geom_mesh::TriMesh;
use geom_model::{GeometryGraph, GeometryGraphBuilder, GeometryNode, NodeId};

use super::*;

#[derive(Debug)]
struct FakeExecutor {
    device: GpuDeviceDescriptor,
}

#[derive(Debug)]
struct RejectingOptionsExecutor {
    inner: FakeExecutor,
    compile_calls: Arc<AtomicUsize>,
}

impl GpuGraphExecutor for FakeExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        &self.device
    }

    fn validate_options(&self, _options: &ExecutionOptions) -> GeomResult<()> {
        Ok(())
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

#[derive(Debug)]
struct WrongCardinalityExecutor(FakeExecutor);

impl GpuGraphExecutor for WrongCardinalityExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        self.0.device()
    }

    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()> {
        self.0.validate_options(options)
    }

    fn compile_batch(
        &self,
        _graph: &GeometryGraph,
        _roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        Ok(vec![TriMesh::default(), TriMesh::default()])
    }
}

impl GpuGraphExecutor for RejectingOptionsExecutor {
    fn device(&self) -> &GpuDeviceDescriptor {
        self.inner.device()
    }

    fn validate_options(&self, _options: &ExecutionOptions) -> GeomResult<()> {
        Err(GeomError::Unsupported {
            backend: self.inner.device.id,
            operation: Operation::GraphCompilation,
        })
    }

    fn compile_batch(
        &self,
        _graph: &GeometryGraph,
        _roots: &[NodeId],
        _options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        self.compile_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
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

#[test]
fn adapter_rejects_foreign_roots_before_dispatch() {
    let compiler = GpuCompiler::new(fake_executor("root-validation", true));
    let (graph, _) = point_graph(Vec3::ZERO);
    let (_, foreign_root) = point_graph(Vec3::ONE);

    assert!(matches!(
        compiler.compile(
            &graph,
            foreign_root,
            &ExecutionOptions::new(Tolerance::METRE),
        ),
        Err(GeomError::InvalidInput { .. })
    ));
}

#[test]
fn adapter_enforces_one_result_per_requested_root() {
    let compiler = GpuCompiler::new(WrongCardinalityExecutor(fake_executor(
        "wrong-cardinality",
        true,
    )));
    let (graph, root) = point_graph(Vec3::ZERO);

    assert!(matches!(
        compiler.compile_batch(&graph, &[root], &ExecutionOptions::new(Tolerance::METRE),),
        Err(GeomError::BackendContractViolation { backend, .. })
            if backend == BackendId::new("wrong-cardinality")
    ));
}

#[test]
fn adapter_runs_executor_policy_validation_before_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let compiler = GpuCompiler::new(RejectingOptionsExecutor {
        inner: fake_executor("policy-rejection", true),
        compile_calls: calls.clone(),
    });
    let (graph, root) = point_graph(Vec3::ZERO);

    assert!(matches!(
        compiler.compile(&graph, root, &ExecutionOptions::new(Tolerance::METRE)),
        Err(GeomError::Unsupported { backend, .. })
            if backend == BackendId::new("policy-rejection")
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn adapter_rejects_incompatible_device_preferences() {
    let compiler = GpuCompiler::new(fake_executor("gpu-only", true));
    let (graph, root) = point_graph(Vec3::ZERO);
    let options = ExecutionOptions::new(Tolerance::METRE).with_device(DevicePreference::Cpu);

    assert!(matches!(
        compiler.compile(&graph, root, &options),
        Err(GeomError::Unsupported {
            backend,
            operation: Operation::GraphCompilation,
        }) if backend == BackendId::new("gpu-only")
    ));
}

fn fake_executor(id: &'static str, float64: bool) -> FakeExecutor {
    FakeExecutor {
        device: GpuDeviceDescriptor {
            id: BackendId::new(id),
            name: "test".to_owned(),
            api: "mock".to_owned(),
            features: GpuFeatures {
                float64,
                subgroups: false,
                unified_memory: true,
                max_workgroup_size: 64,
            },
        },
    }
}

fn point_graph(point: Vec3) -> (GeometryGraph, NodeId) {
    let mut builder = GeometryGraphBuilder::new();
    let root = builder.push(GeometryNode::Point3(point)).expect("root");
    (builder.finish(vec![root]).expect("graph"), root)
}

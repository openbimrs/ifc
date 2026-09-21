//! A third-party kernel can be plugged in without forking this crate.
//!
//! The claim under test is not "the generic parameter compiles" -- that is
//! trivially true. It is that a backend written *outside* this crate, naming
//! none of its internals, is actually the thing that runs: the mesh a caller
//! receives is the one their own code produced.
//!
//! `StubKernel` stands in for CGAL, OCCT, or a GPU tessellator. It implements
//! the public contract and nothing else.

#![cfg(feature = "compile")]

use std::sync::atomic::{AtomicUsize, Ordering};

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomResult,
};
use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, NodeId};
use ifc_geometry::compile::compile_product_mesh_with;
#[cfg(feature = "compile-reference-backend")]
use ifc_geometry::compile::{compile_product_mesh, default_backend};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// A geometry kernel that lives entirely outside `ifc-geometry`.
///
/// It returns a mesh no real compiler would: one degenerate triangle with a
/// recognisable vertex. If this mesh comes back, the caller's backend ran.
#[derive(Debug, Default)]
struct StubKernel {
    calls: AtomicUsize,
}

/// A position no real compilation of the fixture would produce.
fn sentinel() -> Point3 {
    Point3::new(42.0, 43.0, 44.0)
}

impl Backend for StubKernel {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(BackendId::new("stub-kernel"), ExecutionTarget::PortableCpu)
    }
}

impl MeshCompiler for StubKernel {
    fn compile_mesh(
        &self,
        _graph: &GeometryGraph,
        _root: NodeId,
        _options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut mesh = TriMesh::default();
        mesh.positions.push(sentinel());
        Ok(mesh)
    }
}

/// A product with a body representation, so compilation has something to do.
///
/// The class is not hard-coded: the fixture holds `IfcWallStandardCase`
/// rather than `IfcWall`, and pinning either would make this test a
/// statement about one file instead of about the seam.
fn wall_fixture() -> (ifc_model::Model, EntityId) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/ifclite-geometry/issue_098_wall_W.ifc"
    );
    let bytes = std::fs::read(path).expect("fixture is readable");
    let model = StepCodec.read_bytes(&bytes).expect("fixture parses");
    let product = model
        .ids()
        .find(|id| {
            model.get(*id).is_some_and(|e| {
                let name = e.type_name.to_ascii_uppercase();
                name.starts_with("IFCWALL")
            })
        })
        .expect("the fixture holds a wall");
    (model, product)
}

/// The caller's own compiler is the one that runs.
#[test]
fn a_foreign_backend_is_actually_called() {
    let (model, product) = wall_fixture();
    let kernel = StubKernel::default();

    let mesh = compile_product_mesh_with(&kernel, &model, product, Tolerance::MILLIMETRE)
        .expect("the stub kernel accepts every graph");

    assert_eq!(
        kernel.calls.load(Ordering::SeqCst),
        1,
        "the backend must be invoked exactly once per product"
    );
    let mesh = mesh.expect("the wall has a body representation");
    assert_eq!(
        mesh.positions.first().copied(),
        Some(sentinel()),
        "the returned mesh must be the caller's, not a built-in one"
    );
}

/// One backend instance serves many products.
///
/// A real kernel holds caches or device handles, so rebuilding it per product
/// would be a silent performance bug. The borrow in the signature is what
/// makes reuse possible; this pins it.
#[test]
fn one_backend_instance_serves_repeated_calls() {
    let (model, product) = wall_fixture();
    let kernel = StubKernel::default();

    for _ in 0..3 {
        compile_product_mesh_with(&kernel, &model, product, Tolerance::MILLIMETRE)
            .expect("the stub kernel accepts every graph");
    }

    assert_eq!(
        kernel.calls.load(Ordering::SeqCst),
        3,
        "state accumulates in the caller's instance across calls"
    );
}

/// The convenience entry point still works and does not reach the stub.
///
/// Needs the reference backend, which a BYO-kernel build does not link.
///
/// Swapping must be opt-in: a caller who asks for nothing keeps the
/// documented default rather than inheriting whatever was plugged in
/// elsewhere in the process.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn the_default_path_is_unchanged_by_the_generic_seam() {
    let (model, product) = wall_fixture();

    let direct = compile_product_mesh(&model, product, Tolerance::MILLIMETRE)
        .expect("the default backend compiles the fixture");
    let explicit =
        compile_product_mesh_with(&default_backend(), &model, product, Tolerance::MILLIMETRE)
            .expect("naming the default explicitly is the same call");

    let (direct, explicit) = (
        direct.expect("wall has a body"),
        explicit.expect("wall has a body"),
    );
    assert_eq!(
        direct.positions.len(),
        explicit.positions.len(),
        "compile_product_mesh must equal compile_product_mesh_with(default_backend())"
    );
    assert_ne!(
        direct.positions.first().copied(),
        Some(sentinel()),
        "the default path must not pick up a foreign backend"
    );
}

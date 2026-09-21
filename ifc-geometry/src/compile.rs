//! Mesh compilation: run the lowered DAG through a geometry backend.
//!
//! Gated behind the opt-in `compile` feature. ADR 0004 keeps this crate a
//! bridge, and compilation is the one narrow exception: it selects an
//! execution provider and a memory budget, which are application policy
//! rather than properties of the file. Nothing here is reachable in a
//! default build.
//!
//! The bridge still implements no geometry. It calls a compiler that
//! already exists and translates the refusal into IFC terms.
//!
//! # Bringing your own kernel
//!
//! Every entry point is generic over [`MeshCompiler`], so a caller may bring
//! CGAL, OCCT, a GPU tessellator, or a research prototype without forking
//! this crate. Two granularities are supported:
//!
//! * **Whole-kernel.** Implement [`MeshCompiler`] over your own engine and
//!   pass it to [`compile_product_mesh_with`]. Enable `compile` alone and
//!   none of the reference execution path is even linked.
//! * **Per-area.** Keep the reference compiler's traversal and swap only the
//!   area a kernel does well, e.g. `ReferenceMeshCompiler::new(MyBoolean)`
//!   to replace boolean evaluation while retaining everything else. That
//!   needs the `compile-reference-backend` feature for the traversal.
//!
//! Selection is the application's, never the file's: two backends given the
//! same model must agree on what the file means, so choosing between them is
//! a question of speed, robustness, or licence -- which is why this module
//! refuses to pick for you beyond the documented default.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use crate::units;

#[cfg(feature = "compile-reference-backend")]
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
#[cfg(feature = "compile-reference-backend")]
use axiolid_mesh_compile::ReferenceMeshCompiler;

/// The backend used when a caller expresses no preference.
///
/// Named and returned rather than constructed inline so a caller can ask
/// what the default *is* -- to log it beside a benchmark, or to compare
/// against it -- without hard-coding the same type this module does.
#[cfg(feature = "compile-reference-backend")]
#[must_use]
pub fn default_backend() -> ReferenceMeshCompiler<BoolmeshBoolean> {
    ReferenceMeshCompiler::new(BoolmeshBoolean)
}

/// Compile one product's body representation into triangles.
///
/// Uses [`default_backend`]. Call [`compile_product_mesh_with`] to supply
/// your own.
///
/// Returns `Ok(None)` when the product has no body representation at all,
/// which is ordinary -- a spatial container or an annotation-only product is
/// not an error.
///
/// # Tolerance
///
/// Lowering converts every length to metres, and the backend reads tolerance
/// in the model's current length unit, so [`Tolerance::MILLIMETRE`] (1e-3) is
/// a millimetre here regardless of what the file declared. Deriving a
/// tolerance from the file's unit scale would double-apply the conversion.
///
/// # Errors
///
/// Returns [`GeometryError::CompilationRefused`] when the backend cannot
/// produce a mesh, and any lowering error the representation raises first.
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_mesh(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<TriMesh>> {
    compile_product_mesh_with(&default_backend(), model, product, tolerance)
}

/// Compile one product's body representation with a caller-supplied backend.
///
/// The backend is borrowed, so one instance serves a whole run: building a
/// compiler per product would discard whatever caches or device handles an
/// implementation holds.
///
/// # Errors
///
/// Returns [`GeometryError::CompilationRefused`] when the backend cannot
/// produce a mesh, and any lowering error the representation raises first.
///
/// # Examples
///
/// Swapping only boolean evaluation, keeping the reference traversal:
///
/// ```no_run
/// # #[cfg(feature = "compile-reference-backend")] {
/// # use axiolid_core::Tolerance;
/// # use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
/// # use axiolid_mesh_compile::ReferenceMeshCompiler;
/// # use ifc_geometry::compile::compile_product_mesh_with;
/// # use ifc_model::{EntityId, Model};
/// # fn run(model: &Model, product: EntityId) -> Result<(), Box<dyn std::error::Error>> {
/// let backend = ReferenceMeshCompiler::new(BoolmeshBoolean);
/// let mesh = compile_product_mesh_with(&backend, model, product, Tolerance::MILLIMETRE)?;
/// # let _ = mesh;
/// # Ok(())
/// # }
/// # }
/// ```
pub fn compile_product_mesh_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<TriMesh>> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let Some(root) =
        lower_product_representation(&mut session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    let lowered = session.finish(root)?;

    let options = ExecutionOptions::new(tolerance);
    backend
        .compile_mesh(&lowered.graph, lowered.root, &options)
        .map(Some)
        .map_err(|error| GeometryError::CompilationRefused {
            entity: product,
            reason: format!("{error:?}"),
        })
}

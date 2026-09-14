//! Mesh compilation: run the lowered DAG through an Axiolid provider.
//!
//! Gated behind the opt-in `compile` feature. ADR 0004 keeps this crate a
//! bridge, and compilation is the one narrow exception: it selects an
//! execution provider and a memory budget, which are application policy
//! rather than properties of the file. Nothing here is reachable in a
//! default build.
//!
//! The bridge still does not implement geometry. It calls a compiler that
//! already exists and translates the refusal into IFC terms.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshCompiler;
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use crate::units;

/// Compile one product's body representation into triangles.
///
/// Returns `Ok(None)` when the product has no body representation at all,
/// which is ordinary -- a spatial container or an annotation-only product is
/// not an error.
///
/// # Tolerance
///
/// Lowering converts every length to metres, and Axiolid reads tolerance in
/// the model's current length unit, so [`Tolerance::MILLIMETRE`] (1e-3) is a
/// millimetre here regardless of what the file declared. Deriving a tolerance
/// from the file's unit scale would double-apply the conversion.
pub fn compile_product_mesh(
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

    let compiler = ReferenceMeshCompiler::new(BoolmeshBoolean);
    let options = ExecutionOptions::new(tolerance);
    compiler
        .compile_mesh(&lowered.graph, lowered.root, &options)
        .map(Some)
        .map_err(|error| GeometryError::CompilationRefused {
            entity: product,
            reason: format!("{error:?}"),
        })
}

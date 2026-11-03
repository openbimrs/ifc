//! Per-layer meshes: one mesh for each material layer of a product's Body
//! (#455).

use axiolid_contracts::ExecutionOptions;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::GeometryNode;
use ifc_model::{EntityId, Model};

use super::{attribute_net_refusal, refused, MeshClosure, Tolerance, TriMesh};
use crate::error::{GeometryError, GeometryResult};
use crate::lower::{lower_product_layers, LayerBody, LayerCut, LoweringSession};
use crate::units;

/// One layer of a split Body, meshed.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayerMesh {
    /// World-space triangles in metres. Empty where the layer has no volume
    /// in the Body (a Body that does not reach it).
    pub mesh: TriMesh,
    /// As [`CompiledMesh::closure`](super::CompiledMesh::closure).
    pub closure: MeshClosure,
}

/// A product's Body split by a [`LayerCut`], one mesh per layer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LayerMeshes {
    /// One entry per layer of the cut, in order; `None` for a zero-thick
    /// layer.
    pub parts: Vec<Option<LayerMesh>>,
    /// The openings subtracted, in ascending id, for [`LayerBody::Net`].
    pub openings: Vec<EntityId>,
    /// The openings the Body already carries, taken as applied, for
    /// [`LayerBody::Net`].
    pub taken_as_applied: Vec<EntityId>,
}

/// Mesh each layer of a product's Body, split by `cut` (#455).
///
/// Uses [`default_backend`](super::default_backend);
/// [`compile_product_layer_meshes_with`] takes your own.
///
/// # Errors
///
/// As [`compile_product_layer_meshes_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_layer_meshes(
    model: &Model,
    product: EntityId,
    cut: &LayerCut,
    body: LayerBody,
    tolerance: Tolerance,
) -> GeometryResult<Option<LayerMeshes>> {
    compile_product_layer_meshes_with(
        &super::default_backend(),
        model,
        product,
        cut,
        body,
        tolerance,
    )
}

/// Mesh each layer of a product's Body with a caller-supplied backend.
///
/// Each part is the Body cut by the inner boundaries of its layer, in the
/// product's frame; the outermost layers extend, so the parts add up to the
/// Body (see [`lower_product_layers`]). The Body is gross, or net on request.
///
/// # Return values
///
/// `Ok(None)` when the product has no Body representation.
///
/// # Cost
///
/// The success path compiles each part once; the backend's cache shares the
/// Body across them. Only a refusal pays more: the Body is compiled alone, to
/// tell a refusal of the Body from one of a layer.
///
/// # Errors
///
/// As [`lower_product_layers`]. A refusal the Body alone would raise is
/// reported as the gross or net path reports it (for a net Body,
/// [`GeometryError::OpeningNotSubtracted`]); otherwise it is
/// [`GeometryError::LayerNotSplit`] naming the first layer refused.
pub fn compile_product_layer_meshes_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    cut: &LayerCut,
    body: LayerBody,
    tolerance: Tolerance,
) -> GeometryResult<Option<LayerMeshes>> {
    let scale = units::resolve(model)?;
    let mut session = LoweringSession::new(model, &scale);
    let Some(layered) = lower_product_layers(&mut session, product, cut, body)? else {
        return Ok(None);
    };
    let present: Vec<_> = layered.parts.iter().flatten().copied().collect();
    let root = session.node_for(product, GeometryNode::Collection(present))?;
    let lowered = session.finish(root)?;
    let options = ExecutionOptions::new(tolerance);

    let mut parts = Vec::with_capacity(layered.parts.len());
    for (layer, part) in layered.parts.iter().enumerate() {
        let Some(part) = part else {
            parts.push(None);
            continue;
        };
        match backend.compile_mesh_reported(&lowered.graph, *part, &options) {
            Ok(outcome) => parts.push(Some(LayerMesh {
                mesh: outcome.mesh,
                closure: outcome.closure,
            })),
            Err(error) => {
                // Is it the Body or the cut that the backend refuses?
                let body_error = match &layered.net {
                    Some(net) => backend
                        .compile_mesh_reported(&lowered.graph, net.root, &options)
                        .err()
                        .map(|_| {
                            attribute_net_refusal(
                                backend,
                                &lowered,
                                net,
                                &options,
                                product,
                                error.clone(),
                            )
                        }),
                    None => backend
                        .compile_mesh_reported(&lowered.graph, layered.body, &options)
                        .err()
                        .map(|body| refused(product, body, &lowered.provenance)),
                };
                return Err(body_error.unwrap_or_else(|| GeometryError::LayerNotSplit {
                    product,
                    layer,
                    cause: Box::new(refused(product, error, &lowered.provenance)),
                }));
            }
        }
    }
    let (openings, taken_as_applied) = layered.net.as_ref().map_or_else(
        || (Vec::new(), Vec::new()),
        |net| {
            (
                net.openings(),
                net.taken_as_applied.iter().map(|a| a.opening).collect(),
            )
        },
    );
    Ok(Some(LayerMeshes {
        parts,
        openings,
        taken_as_applied,
    }))
}

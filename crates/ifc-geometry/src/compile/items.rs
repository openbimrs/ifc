//! Per-item meshes: one gross mesh for each leaf item of a Body (#656).

use axiolid_contracts::ExecutionOptions;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::GeometryNode;
use ifc_model::{EntityId, Model};

use super::{refused, MeshClosure, Tolerance, TriMesh};
use crate::error::GeometryResult;
use crate::lower::items::lower_product_leaves;
use crate::lower::LoweringSession;
use crate::units;

/// One leaf item of a product's Body, meshed on its own (#656).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ItemMesh {
    /// The representation item; never an `IfcMappedItem`.
    pub item: EntityId,
    /// The `IfcMappedItem`s it was reached through, outermost first; empty
    /// when it is authored directly in the Body.
    pub mapped_by: Vec<EntityId>,
    /// World-space triangles in metres, this occurrence's placement applied.
    pub mesh: TriMesh,
    /// As [`CompiledMesh::closure`](super::CompiledMesh::closure).
    pub closure: MeshClosure,
}

/// Mesh each leaf item of a product's Body on its own, gross (#656).
///
/// Uses [`default_backend`](super::default_backend);
/// [`compile_product_item_meshes_with`] takes your own.
///
/// # Errors
///
/// As [`compile_product_item_meshes_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_product_item_meshes(
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<Vec<ItemMesh>>> {
    compile_product_item_meshes_with(&super::default_backend(), model, product, tolerance)
}

/// Mesh each leaf item of a product's Body with a caller-supplied backend.
///
/// A viewer that colours per item needs one mesh per styled item, in the
/// order its colours come: index `i` here and index `i` of
/// `ColourResolver::item_colours` name the same `item`.
///
/// # Return values
///
/// * `Ok(None)`: the product has no Body representation, in the cases where
///   [`compile_product_mesh_with`](super::compile_product_mesh_with) returns
///   `None`.
/// * `Ok(Some(vec![]))`: the Body's items expand to no leaves. Unlike this,
///   `compile_product_mesh_with` returns `Ok(None)` for an empty `Items`.
///
/// # Order
///
/// One entry per leaf, in authored `Items` order, depth first, every
/// `IfcMappedItem` expanded through `MappingSource.MappedRepresentation`.
/// Every other item is a leaf, including booleans, CSG and geometric sets.
/// The representation is the one `select_product_representation` picks for
/// the Body. Two occurrences of one map give two entries with the same
/// `item` and different `mapped_by`.
///
/// # Geometry
///
/// Each leaf compiles the nodes and transforms the whole Body does, so for a
/// single-item Body without maps the mesh equals
/// [`compile_product_mesh_with`](super::compile_product_mesh_with)'s bit for
/// bit. For several items the triangle counts sum to the product mesh's and
/// the bounds join to its bounds. The meshes are gross: openings are not
/// subtracted (ADR 0014). A curve leaf is compiled as the backend treats a
/// one-item Body of that curve.
///
/// # Errors
///
/// All or nothing; no partial list is returned. Any lowering error of any
/// leaf is returned unchanged (`MissingEntity`, `Unsupported`, `CyclicChain`
/// or `ChainTooDeep` of kind `"mapped item"`, units errors), as
/// `compile_product_mesh_with` raises them. A backend refusal of any leaf is
/// [`GeometryError::CompilationRefused`](crate::error::GeometryError::CompilationRefused)
/// naming `product`, or its seam-window variants.
///
/// # Cost
///
/// One lowering per product and one compilation per leaf. A map's source is
/// lowered once per product and instanced.
pub fn compile_product_item_meshes_with<B: MeshCompiler>(
    backend: &B,
    model: &Model,
    product: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Option<Vec<ItemMesh>>> {
    let scale = units::resolve(model)?;
    let mut session = LoweringSession::new(model, &scale);
    let Some(leaves) = lower_product_leaves(&mut session, product)? else {
        return Ok(None);
    };
    if leaves.is_empty() {
        return Ok(Some(Vec::new()));
    }
    // `finish` needs one root; the leaves are compiled from their own.
    let root = if let [only] = leaves.as_slice() {
        only.root
    } else {
        let mut distinct: Vec<_> = leaves.iter().map(|leaf| leaf.root).collect();
        distinct.sort();
        distinct.dedup();
        session.node_for(product, GeometryNode::Collection(distinct))?
    };
    let lowered = session.finish(root)?;

    let options = ExecutionOptions::new(tolerance);
    leaves
        .into_iter()
        .map(|leaf| {
            backend
                .compile_mesh_reported(&lowered.graph, leaf.root, &options)
                .map(|outcome| ItemMesh {
                    item: leaf.item,
                    mapped_by: leaf.mapped_by,
                    mesh: outcome.mesh,
                    closure: outcome.closure,
                })
                .map_err(|error| refused(product, error, &lowered.provenance))
        })
        .collect::<GeometryResult<Vec<_>>>()
        .map(Some)
}

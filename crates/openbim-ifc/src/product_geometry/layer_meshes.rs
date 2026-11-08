//! One mesh per material layer of a layered wall, slab or plate (#455).
//!
//! [`material_layer_bounds`] says where each layer's planes lie in the
//! product's object frame; `ifc-geometry` cuts the Body by them
//! ([`compile_product_layer_meshes_with`]). This module joins the two, since
//! neither crate may depend on the other, and puts the result in the object
//! frame the `bounds` are stated in: the compiled world positions mapped back
//! through the inverse of the Body's representation frame in `f64` and only
//! then narrowed to `f32`. That frame is the product's placement with the
//! representation context's `WorldCoordinateSystem` above it, and `world` is
//! that frame, so `world` applied to a layer's positions is its world position
//! and a position compares directly with `bounds` whatever the context.
//! (Before #706 the positions were mapped back through the placement alone and
//! carried the context frame, as a [`ProductMesh`](super::ProductMesh) still
//! does; the two agree when the context frame is the identity.)
//!
//! The Body is gross unless [`LayerBody::Net`] is asked for (ADR 0014). The
//! outermost layers extend to the Body's faces and only the inner boundaries
//! cut, so the layer volumes add up to the Body (ADR 0014, amendment for
//! #455). A zero-thick layer has no part: an empty mesh, closure
//! [`MeshClosure::Unknown`]. A layer the Body does not reach is a part with no
//! volume: an empty mesh too, typically with closure [`MeshClosure::Solid`].

#![cfg(all(feature = "material", feature = "mesh"))]

use ifc_geometry::compile::{
    compile_product_layer_meshes_with, default_backend, MeshClosure, Tolerance,
};
use ifc_geometry::lower::LayerCut;
use ifc_geometry::{product_representation_frame, units, RepresentationPurpose, Transform};
use ifc_model::{EntityId, Model};

pub use ifc_geometry::lower::LayerBody;

use super::layers::{material_layer_bounds, LayerBounds, LayerBoundsError};
use super::mesh::Inverse;

/// A layered product's Body, one mesh per material layer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MaterialLayerMeshes {
    /// The planes the Body was cut by.
    pub bounds: LayerBounds,
    /// The Body's frame in metres: the product's world placement with the
    /// representation context's `WorldCoordinateSystem` above it, equal to the
    /// placement when that frame is the identity. A vertex's world position is
    /// `world` applied to its entry in a layer's `positions`, and those
    /// positions are in the object frame of `bounds`.
    pub world: Transform,
    /// One entry per layer, `layers[k]` for `bounds.layers[k]`.
    pub layers: Vec<MaterialLayerMesh>,
    /// The openings subtracted, in ascending id, for [`LayerBody::Net`].
    pub openings: Vec<EntityId>,
    /// The openings the Body already carries, taken as applied, for
    /// [`LayerBody::Net`].
    pub taken_as_applied: Vec<EntityId>,
}

/// One material layer's part of the Body.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MaterialLayerMesh {
    /// The layer's index in `IfcMaterialLayerSet.MaterialLayers`.
    pub index: usize,
    /// The `IfcMaterialLayer`.
    pub layer: EntityId,
    /// Its `IfcMaterial`, or `None` for a layer without one (an air gap).
    pub material: Option<EntityId>,
    /// Vertex positions, `x, y, z` per vertex, in metres, in the
    /// object frame `bounds` is stated in: `world` applied to them is world
    /// space.
    /// Empty for a zero-thick layer (no part) and for a layer the Body does
    /// not reach (a part with no volume).
    pub positions: Vec<f32>,
    /// Triangles as indices into the vertices, three per triangle.
    pub indices: Vec<u32>,
    /// Whether the part bounds a solid. A zero-thick layer, which has no
    /// part, is [`MeshClosure::Unknown`]; a part with no volume is an empty
    /// mesh that is typically [`MeshClosure::Solid`].
    pub closure: MeshClosure,
}

/// The Body of `product` split into one mesh per material layer.
///
/// `Ok(None)` when the product's material is not an
/// `IfcMaterialLayerSetUsage`, or the product has no Body.
///
/// # Errors
///
/// [`LayerBoundsError`] as [`material_layer_bounds`] raises it; a refusal to
/// split or compile the Body (the net refusals, or the layer a backend
/// refuses) is [`LayerBoundsError::Geometry`], never a Body with the layer
/// left out.
pub fn material_layer_meshes(
    model: &Model,
    product: EntityId,
    body: LayerBody,
) -> Result<Option<MaterialLayerMeshes>, LayerBoundsError> {
    let Some(bounds) = material_layer_bounds(model, product)? else {
        return Ok(None);
    };
    let cut = LayerCut::new(bounds.normal, bounds.boundaries.clone());
    let Some(split) = compile_product_layer_meshes_with(
        &default_backend(),
        model,
        product,
        &cut,
        body,
        Tolerance::MILLIMETRE,
    )?
    else {
        return Ok(None);
    };
    let scale = units::resolve(model)?;
    let world = product_representation_frame(model, &scale, product, RepresentationPurpose::Body)?
        .expect("the Body was split, so the product has one");
    let to_local = Inverse::checked(model, product, &world)?;

    let mut layers = Vec::with_capacity(bounds.layers.len());
    for (layer, part) in bounds.layers.iter().zip(&split.parts) {
        let (positions, indices, closure) = match part {
            Some(part) => (
                to_local.narrow(part.mesh.positions.iter().map(|p| [p.x, p.y, p.z])),
                part.mesh.indices.clone(),
                part.closure,
            ),
            None => (Vec::new(), Vec::new(), MeshClosure::Unknown),
        };
        layers.push(MaterialLayerMesh {
            index: layer.index,
            layer: layer.layer,
            material: layer.material,
            positions,
            indices,
            closure,
        });
    }
    Ok(Some(MaterialLayerMeshes {
        bounds,
        world,
        layers,
        openings: split.openings,
        taken_as_applied: split.taken_as_applied,
    }))
}

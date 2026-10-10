//! The neutral representation per product, as a value (#367).
//!
//! The level between placements and meshes the language bindings carry
//! (ADR 0021): each product's Body lowered into Axiolid's format-neutral
//! [`GeometryGraph`], exact (extrusions, sweeps, B-splines, unevaluated
//! booleans), for a host to evaluate with its own kernel. It computes
//! nothing: the graph is the one [`product_meshes`](crate::product_meshes)
//! hands the reference backend, and the encodings are Axiolid's versioned
//! wire format (axiolid/kernel#267, Axiolid ADR 0085), reached as
//! [`ifc_geometry::wire`] and `GeometryGraph::to_json` / `to_cbor`. A
//! payload's envelope carries the lowest version its content needs: 1.0,
//! or 1.1 when a station carries a seam-snapping window (#423); readers on
//! `axiolid-model` 0.3.8 or older refuse 1.1. `axiolid-model` 0.3.9 labels
//! every payload 1.1 (axiolid/kernel#297).
//!
//! - **The graph is in world coordinates, metres.** Lowering composes the
//!   product's placement chain into the graph (its `Instance` transforms),
//!   exactly as the compiled meshes are in world coordinates before the
//!   mesh level re-expresses them. The [`ProductGraph::world`] placement
//!   rides along, the same answer
//!   [`product_placements`](crate::product_placements) gives, for a host
//!   that wants the product's own frame; it is never to be applied again.
//!   The wire format carries `f64` bit-exactly, so a georeferenced site
//!   loses nothing.
//! - **One refusal per product.** A product whose placement or lowering is
//!   refused keeps its typed [`GeometryError`]; the others still lower.

#![cfg(feature = "geometry-wire")]

use ifc_geometry::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use ifc_geometry::{
    geometric_products, products_world_transforms, units, GeometryError, GeometryGraph,
    GeometryResult, Transform,
};
use ifc_model::{EntityId, Model};

/// One product's Body as Axiolid's neutral geometry graph.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductGraph {
    /// The product's world placement in metres. The graph already has it
    /// applied.
    pub world: Transform,
    /// The lowered Body with one root, in world coordinates in metres;
    /// `None` when the product has no Body representation (an axis or a
    /// footprint only).
    pub graph: Option<GeometryGraph>,
}

/// The Body graph of each of `products`, or, with `None`, of every product
/// that has a shape, in id order.
///
/// A product with no Body representation is not a failure: its graph is
/// `None`, and it is still placed. A product whose placement or lowering is
/// refused carries the error. Encode a graph with `GeometryGraph::to_json`
/// or `to_cbor` (feature `geometry-wire`); a graph holding a non-finite
/// number is refused there, as
/// [`WireError::NonFinite`](ifc_geometry::wire::WireError::NonFinite).
pub fn product_graphs(
    model: &Model,
    products: Option<&[EntityId]>,
) -> Vec<(EntityId, GeometryResult<ProductGraph>)> {
    let products = products.map_or_else(|| geometric_products(model), <[EntityId]>::to_vec);
    let scale = units::resolve(model);
    products_world_transforms(model, &scale, products)
        .into_iter()
        .map(|(product, world)| {
            let graph = world.and_then(|world| {
                Ok(ProductGraph {
                    world,
                    graph: lower_body(model, &scale, product)?,
                })
            });
            (product, graph)
        })
        .collect()
}

/// The product's Body lowered into a fresh graph, as the mesh compiler
/// lowers it.
fn lower_body(
    model: &Model,
    scale: &ifc_geometry::UnitScale,
    product: EntityId,
) -> Result<Option<GeometryGraph>, GeometryError> {
    let mut session = LoweringSession::new(model, scale);
    let Some(root) =
        lower_product_representation(&mut session, product, RepresentationPurpose::Body)?
    else {
        return Ok(None);
    };
    Ok(Some(session.finish(root)?.graph))
}

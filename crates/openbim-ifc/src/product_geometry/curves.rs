//! Polylines per product, compiled by the reference backend (#458).
//!
//! The curve counterpart of [`product_meshes`](super::product_meshes): an
//! `Axis`, a `FootPrint`, the curves of an `IfcAnnotation` have no area, so
//! they have no triangles. Each curve of the representation the selector
//! picks becomes one polyline with the backend's certified chord bound,
//! positioned as a mesh is -- `f32` coordinates relative to the product's
//! world placement, mapped back in `f64` first -- so a host draws both with
//! one matrix. The tolerance is [`Tolerance::MILLIMETRE`], the chord budget.
//!
//! Trimmed, composite, offset and intrinsic curves are refused by name
//! ([`GeometryError::PolylineUnsupported`](ifc_geometry::GeometryError::PolylineUnsupported))
//! until the reference compiler polylines them (#506).

#![cfg(feature = "mesh")]

use ifc_geometry::compile::{compile_product_polylines_with, default_backend, Tolerance};
use ifc_geometry::{
    geometric_products, products_world_transforms, units, GeometryResult, RepresentationPurpose,
    Transform,
};
use ifc_model::{EntityId, Model};

use super::mesh::Inverse;

/// One curve as a polyline.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductPolyline {
    /// The IFC curve it was compiled from, when the lowering names one.
    pub curve: Option<EntityId>,
    /// `x, y, z` per vertex, metres, in the product's placement frame, in
    /// the curve's direction. A closed curve ends on its first vertex.
    pub positions: Vec<f32>,
    /// Whether the curve is closed.
    pub closed: bool,
    /// The certified bound, in metres, on the distance from every point of
    /// the exact curve to the polyline: `0` for a polyline the file
    /// authored, at most `chord_requested` otherwise.
    pub chord_bound: f64,
    /// The chord budget it was compiled to, in metres.
    pub chord_requested: f64,
}

/// One product's curves for a representation purpose.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductCurves {
    /// The product's world placement in metres; a vertex's world position
    /// is `world` applied to it.
    pub world: Transform,
    /// The representation's curves in authored order; empty when it has
    /// none or the product has no representation for the purpose.
    pub polylines: Vec<ProductPolyline>,
}

/// The curves of each product's representation for `purpose` as
/// polylines (#458), or, with `None`, of every product that has a shape,
/// in id order.
///
/// `purpose` picks the representation as the selector does
/// ([`select_product_representation`](ifc_geometry::select_product_representation)):
/// [`RepresentationPurpose::Plan`] the best drawing curves (`Plan`, then
/// `Annotation`, `FootPrint`, `Axis`, preferring a `PLAN_VIEW` context),
/// or one identifier: `Axis`, `FootPrint`, `Annotation`. Its surfaces and
/// solids are not here: [`product_meshes_for`](super::product_meshes_for)
/// meshes them. A product with no such representation has no polylines and
/// no refusal. One refused curve refuses the product.
pub fn product_curves(
    model: &Model,
    products: Option<&[EntityId]>,
    purpose: RepresentationPurpose,
) -> Vec<(EntityId, GeometryResult<ProductCurves>)> {
    let products = products.map_or_else(|| geometric_products(model), <[EntityId]>::to_vec);
    let scale = units::resolve(model);
    let backend = default_backend();
    products_world_transforms(model, &scale, products)
        .into_iter()
        .map(|(product, world)| {
            let curves = world.and_then(|world| {
                let compiled = compile_product_polylines_with(
                    &backend,
                    model,
                    product,
                    purpose,
                    Tolerance::MILLIMETRE,
                )?
                .unwrap_or_default();
                if compiled.is_empty() {
                    return Ok(ProductCurves {
                        world,
                        polylines: Vec::new(),
                    });
                }
                let to_local = Inverse::checked(model, product, &world)?;
                let polylines = compiled
                    .into_iter()
                    .map(|polyline| ProductPolyline {
                        curve: polyline.curve,
                        positions: to_local.narrow(polyline.points.iter().map(|p| [p.x, p.y, p.z])),
                        closed: polyline.closed,
                        chord_bound: polyline.deviation.bound,
                        chord_requested: polyline.deviation.requested,
                    })
                    .collect();
                Ok(ProductCurves { world, polylines })
            });
            (product, curves)
        })
        .collect()
}

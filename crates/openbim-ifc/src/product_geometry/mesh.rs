//! Triangle meshes per product, compiled by the reference backend (#328).
//!
//! The second level the language bindings carry (ADR 0021), opt-in behind
//! the `mesh` feature because it links an execution provider: ADR 0004
//! admits one, in `ifc-geometry` only and never by default, and ADR 0012
//! names the reference backend. This module adds no geometry. It calls
//! [`compile_product_mesh_with`] once per product with one backend instance,
//! and re-expresses the result for a host that draws it:
//!
//! - **Positions relative to the product.** The compiler returns world
//!   coordinates in metres. A georeferenced site sits kilometres from the
//!   origin, where an `f32` -- what a GPU reads -- keeps centimetres at
//!   best, so each vertex is mapped back through the inverse of the
//!   product's world placement in `f64` and only then narrowed. The host
//!   draws `world * position`, keeping the large offset in the `f64`
//!   matrix.
//! - **The placement is the facade's.** It is
//!   [`products_world_transforms`](ifc_geometry::products_world_transforms),
//!   the same answer [`product_placements`](crate::product_placements)
//!   gives, so a change to placement resolution reaches both.
//! - **One refusal per product.** A product the compiler or the lowering
//!   refuses keeps its typed [`GeometryError`]; the others still mesh.
//! - **A colour per product** (#449). Each mesh carries the surface colour
//!   its Body's items agree on, from [`ColourResolver`]: the item's style,
//!   its presentation layer's, else its material's. A colour that cannot be
//!   resolved is its own typed [`ColourError`] and never refuses the mesh.
//!
//! The tolerance is [`Tolerance::MILLIMETRE`]: lowering converts every
//! length to metres, so it is a millimetre whatever unit the file declared
//! (ADR 0004 leaves tolerance to the bridge for exactly that reason).
//!
//! # Cost
//!
//! Meshing a subset costs the subset (#608), also inside `Model::preview`:
//! units, placements, representations and colours are all found through
//! the entities that reference the product. The first Body mesh of a model
//! makes it keep its reverse index (one scan, see [`ColourResolver`]); the
//! other purposes never touch it.

#![cfg(feature = "mesh")]

use ifc_geometry::compile::{
    compile_product_mesh_for_with, default_backend, MeshClosure, Tolerance,
};
use ifc_geometry::{
    geometric_products, products_world_transforms, units, GeometryError, GeometryResult,
    RepresentationPurpose, Transform,
};
use ifc_model::{EntityId, Model};

use super::colour::{ColourError, ColourResolver, SurfaceColour};

/// One product's Body as triangles.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductMesh {
    /// The product's world placement in metres; a vertex's world position
    /// is `world` applied to its entry in `positions`.
    pub world: Transform,
    /// Vertex positions, `x, y, z` per vertex, in metres in the product's
    /// placement frame.
    pub positions: Vec<f32>,
    /// Triangle corners, three indices into the vertices per triangle.
    pub indices: Vec<u32>,
    /// The surface colour the Body's items agree on
    /// ([`ColourResolver::product_colour`]); `Ok(None)` when nothing
    /// colours it, and the typed reason when it cannot be resolved. Only
    /// the Body's colour is resolved: a mesh of another representation
    /// ([`product_meshes_for`]) carries `Ok(None)`.
    pub colour: Result<Option<SurfaceColour>, ColourError>,
    /// Whether the mesh bounds a solid the file declared
    /// ([`MeshClosure::Solid`]) or is a surface ([`MeshClosure::Surface`]:
    /// a surface model, a face, a trimmed surface), which has an area and
    /// no volume (#458). An empty mesh is [`MeshClosure::Unknown`].
    pub closure: MeshClosure,
}

/// The Body mesh of each of `products`, or, with `None`, of every product
/// that has a shape, in id order: [`product_meshes_for`] with
/// [`RepresentationPurpose::Body`].
///
/// A product with no Body representation (an axis or a footprint only) is
/// not a failure: its mesh is empty, and still placed. A product whose
/// placement, lowering or compilation is refused carries the error; a
/// placement whose linear part cannot be inverted is
/// [`GeometryError::Degenerate`].
pub fn product_meshes(
    model: &Model,
    products: Option<&[EntityId]>,
) -> Vec<(EntityId, GeometryResult<ProductMesh>)> {
    product_meshes_for(model, products, RepresentationPurpose::Body)
}

/// The surfaces and solids of each product's representation for
/// `purpose` as one mesh (#458), or, with `None`, of every product that has
/// a shape, in id order.
///
/// The representation is the one the selector picks for `purpose`
/// ([`select_product_representation`](ifc_geometry::select_product_representation)):
/// `Body` (the default of [`product_meshes`]), `Surface`, ... Its curves
/// have no area and are not in the mesh: [`product_curves`](super::product_curves)
/// returns them. A product with no such representation, or one of curves
/// only, has an empty mesh and no refusal. A Body with no curves meshes
/// exactly as before.
pub fn product_meshes_for(
    model: &Model,
    products: Option<&[EntityId]>,
    purpose: RepresentationPurpose,
) -> Vec<(EntityId, GeometryResult<ProductMesh>)> {
    let products = products.map_or_else(|| geometric_products(model), <[EntityId]>::to_vec);
    let scale = match units::resolve(model) {
        Ok(scale) => scale,
        Err(error) => {
            return products
                .into_iter()
                .map(|product| (product, Err(error.clone())))
                .collect();
        }
    };
    let backend = default_backend();
    // Only the Body carries a colour; the other purposes never build the
    // resolver.
    let resolver = (purpose == RepresentationPurpose::Body).then(|| ColourResolver::new(model));
    let colour = |product| match &resolver {
        Some(Ok(resolver)) => resolver.product_colour(product),
        Some(Err(error)) => Err(error.clone()),
        None => Ok(None),
    };
    products_world_transforms(model, &scale, products)
        .into_iter()
        .map(|(product, world)| {
            let mesh = world.and_then(|world| {
                let Some(compiled) = compile_product_mesh_for_with(
                    &backend,
                    model,
                    product,
                    purpose,
                    Tolerance::MILLIMETRE,
                )?
                else {
                    return Ok(ProductMesh {
                        world,
                        positions: Vec::new(),
                        indices: Vec::new(),
                        colour: colour(product),
                        closure: MeshClosure::Unknown,
                    });
                };
                let to_local = Inverse::checked(model, product, &world)?;
                let positions =
                    to_local.narrow(compiled.mesh.positions.iter().map(|p| [p.x, p.y, p.z]));
                Ok(ProductMesh {
                    world,
                    positions,
                    indices: compiled.mesh.indices,
                    colour: colour(product),
                    closure: compiled.closure,
                })
            });
            (product, mesh)
        })
        .collect()
}

/// The inverse of an affine [`Transform`]: `rows` is the inverse of its
/// linear part, applied after removing the translation.
pub(super) struct Inverse {
    rows: [[f64; 3]; 3],
    origin: [f64; 3],
}

impl Inverse {
    /// The inverse of `product`'s placement `world`, or the typed refusal
    /// of one whose linear part cannot be inverted.
    pub(super) fn checked(
        model: &Model,
        product: EntityId,
        world: &Transform,
    ) -> GeometryResult<Self> {
        Self::of(world).ok_or_else(|| GeometryError::Degenerate {
            entity: product,
            type_name: model
                .get(product)
                .map_or_else(String::new, |entity| entity.type_name.to_string()),
            detail: "its world placement cannot be inverted".into(),
        })
    }

    /// World points in metres as `f32` coordinates in the placement's
    /// frame: mapped back in `f64`, then narrowed, on purpose (see the
    /// module docs).
    pub(super) fn narrow(&self, points: impl Iterator<Item = [f64; 3]>) -> Vec<f32> {
        points
            .flat_map(|p| self.apply(p))
            .map(|coordinate| coordinate as f32)
            .collect()
    }

    /// `None` when the linear part is singular or not finite.
    fn of(transform: &Transform) -> Option<Self> {
        let [c0, c1, c2] = transform.basis;
        let r0 = cross(c1, c2);
        let det = dot(c0, r0);
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        let scale = |v: [f64; 3]| [v[0] / det, v[1] / det, v[2] / det];
        Some(Self {
            rows: [scale(r0), scale(cross(c2, c0)), scale(cross(c0, c1))],
            origin: transform.origin,
        })
    }

    fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let v = [
            p[0] - self.origin[0],
            p[1] - self.origin[1],
            p[2] - self.origin[2],
        ];
        self.rows.map(|row| dot(row, v))
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inverse_undoes_a_rotated_scaled_offset_placement() {
        let world = Transform {
            basis: [[0.0, 2.0, 0.0], [-2.0, 0.0, 0.0], [0.0, 0.0, 2.0]],
            origin: [512_000.0, 5_403_000.0, 30.0],
        };
        let inverse = Inverse::of(&world).expect("invertible");
        let local = [1.25, -0.5, 3.0];
        let back = inverse.apply(world.apply(local));
        for (a, b) in back.iter().zip(local) {
            assert!((a - b).abs() < 1e-9, "{back:?} != {local:?}");
        }
    }

    #[test]
    fn a_singular_placement_has_no_inverse() {
        let flat = Transform {
            basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
            origin: [0.0; 3],
        };
        assert!(Inverse::of(&flat).is_none());
    }
}

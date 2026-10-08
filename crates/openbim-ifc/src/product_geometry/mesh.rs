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
//!
//! The tolerance is [`Tolerance::MILLIMETRE`]: lowering converts every
//! length to metres, so it is a millimetre whatever unit the file declared
//! (ADR 0004 leaves tolerance to the bridge for exactly that reason).

#![cfg(feature = "mesh")]

use ifc_geometry::compile::{compile_product_mesh_with, default_backend, Tolerance};
use ifc_geometry::{
    geometric_products, products_world_transforms, units, GeometryError, GeometryResult, Transform,
};
use ifc_model::{EntityId, Model};

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
}

/// The Body mesh of each of `products`, or, with `None`, of every product
/// that has a shape, in id order.
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
    let products = products.map_or_else(|| geometric_products(model), <[EntityId]>::to_vec);
    let scale = units::resolve(model);
    let backend = default_backend();
    products_world_transforms(model, &scale, products)
        .into_iter()
        .map(|(product, world)| {
            let mesh = world.and_then(|world| {
                let Some(mesh) =
                    compile_product_mesh_with(&backend, model, product, Tolerance::MILLIMETRE)?
                else {
                    return Ok(ProductMesh {
                        world,
                        positions: Vec::new(),
                        indices: Vec::new(),
                    });
                };
                let to_local = Inverse::of(&world).ok_or_else(|| GeometryError::Degenerate {
                    entity: product,
                    type_name: model
                        .get(product)
                        .map_or_else(String::new, |entity| entity.type_name.to_string()),
                    detail: "its world placement cannot be inverted".into(),
                })?;
                let positions = mesh
                    .positions
                    .iter()
                    .flat_map(|p| to_local.apply([p.x, p.y, p.z]))
                    // Narrowed after the f64 inverse, on purpose (see above).
                    .map(|coordinate| coordinate as f32)
                    .collect();
                Ok(ProductMesh {
                    world,
                    positions,
                    indices: mesh.indices,
                })
            });
            (product, mesh)
        })
        .collect()
}

/// The inverse of an affine [`Transform`]: `rows` is the inverse of its
/// linear part, applied after removing the translation.
struct Inverse {
    rows: [[f64; 3]; 3],
    origin: [f64; 3],
}

impl Inverse {
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

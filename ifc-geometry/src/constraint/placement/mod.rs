//! Product placement resolution: the answer to "where is this in the world".
//!
//! This is the single most-reused operation in any IFC consumer, and the one
//! most often reimplemented incorrectly. It lives here rather than in `lower`
//! because a 2D drawing needs world coordinates just as much as a 3D
//! tessellation does, and must not have to compile a solid kernel to get them.
//!
//! # Composition order
//!
//! An `IfcLocalPlacement` points at its *parent* through `PlacementRelTo`, so
//! the walk is upward and composition is outermost-last. Reversing that
//! silently mirrors the model about its ancestors.
//!
//! # Units are converted once, at the end
//!
//! Placement coordinates are raw file units. The chain composes unconverted
//! and the composed result is converted once. Converting per link would raise
//! the scale factor to the power of the chain depth -- a millimetre file three
//! levels deep would land a thousand times too far out.

use ifc_model::{EntityId, Model};

use crate::constraint::local::PlacementResolver;
use crate::error::{GeometryError, GeometryResult};
use crate::input::product::Product;
use crate::transform::Transform;
use crate::units::UnitScale;

/// The world transform for one product, in metres.
///
/// Resolves the `IfcLocalPlacement` chain and converts the composed result
/// once. A product with no `ObjectPlacement` is model-space, which the schema
/// allows, so it yields the identity rather than an error.
///
/// Cyclic and over-deep chains are reported as errors rather than hanging or
/// overflowing the stack, so a malformed file cannot lock up a viewer.
///
/// ```no_run
/// # use ifc_model::{EntityId, Model};
/// # use ifc_geometry::{product_world_transform, units::UnitScale};
/// # fn demo(model: &Model, units: &UnitScale, wall: EntityId) {
/// let world = product_world_transform(model, units, wall).unwrap();
/// let [x, y, z] = world.origin;
/// # let _ = (x, y, z);
/// # }
/// ```
///
/// Resolving many products reuses ancestor transforms through
/// [`PlacementResolver`]; see [`products_world_transforms`] for the batch form,
/// which is what a whole-model walk should use.
pub fn product_world_transform(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
) -> GeometryResult<Transform> {
    let mut resolver = PlacementResolver::new();
    resolve_with(&mut resolver, model, units, product)
}

/// World transforms for many products, sharing one placement cache.
///
/// Products in the same storey share the whole storey-building-site tail, so
/// resolving each independently repeats that walk once per element. This
/// resolves them against a single cache instead.
///
/// Errors are per-product: one malformed placement chain does not abort the
/// others, because a viewer should still draw the rest of the building.
pub fn products_world_transforms(
    model: &Model,
    units: &UnitScale,
    products: impl IntoIterator<Item = EntityId>,
) -> Vec<(EntityId, GeometryResult<Transform>)> {
    let mut resolver = PlacementResolver::new();
    products
        .into_iter()
        .map(|product| {
            let resolved = resolve_with(&mut resolver, model, units, product);
            (product, resolved)
        })
        .collect()
}

/// Shared body: resolve one product against a caller-owned resolver.
fn resolve_with(
    resolver: &mut PlacementResolver,
    model: &Model,
    units: &UnitScale,
    product: EntityId,
) -> GeometryResult<Transform> {
    let entity = model.get(product).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: product,
    })?;
    let Some(placement) = Product::new(product, entity).object_placement() else {
        return Ok(Transform::identity());
    };
    let file_units = resolver.world_transform(model, placement)?;
    Ok(file_units.to_metres(units))
}

#[cfg(test)]
mod tests;

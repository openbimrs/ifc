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

#[cfg(feature = "lowering")]
pub(crate) mod linear;

#[cfg(feature = "compile")]
pub mod derive;

#[cfg(feature = "compile")]
pub use derive::CachedPositionPolicy;

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
    resolve_with(
        &mut resolver,
        model,
        units,
        product,
        LinearResolution::cache_only(),
    )
}

/// [`product_world_transform`], deriving an `IfcLinearPlacement` through a
/// caller-supplied evaluator (#353).
///
/// A linear placement without a cached `CartesianPosition` is derived from
/// its `RelativePlacement` with
/// [`derive_linear_placement_transform`](derive::derive_linear_placement_transform);
/// one with a cache is handled by `cached` ([`CachedPositionPolicy`], #354).
/// Every other placement resolves exactly as [`product_world_transform`]
/// does, and the evaluator is never called for it.
///
/// # Errors
///
/// As [`product_world_transform`], plus the derivation's refusals (an
/// `IfcParameterValue` on an alignment centreline is refused by name, #347)
/// and [`GeometryError::CachedPlacementMismatch`] for a stale cache under
/// [`CachedPositionPolicy::Verify`].
#[cfg(feature = "compile")]
pub fn product_world_transform_with_evaluator(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
    evaluator: &dyn axiolid_curve_evaluate_contract::CurveEvaluator,
    cached: CachedPositionPolicy,
) -> GeometryResult<Transform> {
    let mut resolver = PlacementResolver::new();
    resolve_with(
        &mut resolver,
        model,
        units,
        product,
        LinearResolution::derive(evaluator, cached),
    )
}

/// How an `IfcLinearPlacement` is resolved: from its cache only, or, with
/// the `compile` feature, through a caller-supplied evaluator.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LinearResolution<'e> {
    #[cfg(feature = "compile")]
    pub(crate) derivation: Option<derive::Derivation<'e>>,
    #[cfg(not(feature = "compile"))]
    cache_only: std::marker::PhantomData<&'e ()>,
}

impl<'e> LinearResolution<'e> {
    /// The cached `CartesianPosition` or a refusal; no evaluation.
    pub(crate) fn cache_only() -> Self {
        Self {
            #[cfg(feature = "compile")]
            derivation: None,
            #[cfg(not(feature = "compile"))]
            cache_only: std::marker::PhantomData,
        }
    }

    /// Derive through `evaluator`, treating a cache by `cached`.
    #[cfg(feature = "compile")]
    pub(crate) fn derive(
        evaluator: &'e dyn axiolid_curve_evaluate_contract::CurveEvaluator,
        cached: CachedPositionPolicy,
    ) -> Self {
        Self {
            derivation: Some(derive::Derivation { evaluator, cached }),
        }
    }
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
            let resolved = resolve_with(
                &mut resolver,
                model,
                units,
                product,
                LinearResolution::cache_only(),
            );
            (product, resolved)
        })
        .collect()
}

/// Shared body: resolve one product against a caller-owned resolver.
pub(crate) fn resolve_with(
    resolver: &mut PlacementResolver,
    model: &Model,
    units: &UnitScale,
    product: EntityId,
    linear: LinearResolution<'_>,
) -> GeometryResult<Transform> {
    let entity = model.get(product).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: product,
    })?;
    let Some(placement) = Product::new(product, entity).object_placement() else {
        return Ok(Transform::identity());
    };
    // IFC4x3 places linear elements by distance along a curve, which the
    // IfcLocalPlacement walk cannot resolve. Route by type before it.
    #[cfg(feature = "lowering")]
    if linear::is_linear_placement(model, placement) {
        return linear::linear_placement_transform(model, units, placement, linear);
    }
    #[cfg(not(feature = "lowering"))]
    let _ = linear;
    let file_units = resolver.world_transform(model, placement)?;
    Ok(file_units.to_metres(units))
}

#[cfg(test)]
mod tests;

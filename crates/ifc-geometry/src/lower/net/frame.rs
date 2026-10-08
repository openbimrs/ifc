//! The frame a net body is lowered in (#388).
//!
//! # Why not world coordinates
//!
//! A georeferenced site sits hundreds of kilometres from the origin and is
//! turned to grid north. An opening authored flush with its wall states
//! faces that coincide with the wall's; placed in world coordinates, both
//! operands are rounded to the ulp of those large numbers (about 1e-9 m at
//! 5.6e6), the coincidence is lost, and a kernel sees two faces a rounding
//! error apart. In the host's frame the same faces keep what the file
//! states, to a few ulps of metre-sized numbers.
//!
//! So the host's Body is lowered without the host's world placement, each
//! opening's Body by its placement relative to the host's, the `Difference`
//! nodes run there, and one `Instance` with the host's world transform is
//! put above the result.
//!
//! # The relative placement, composed once from the shared chain
//!
//! `host⁻¹ · opening` computed from the two world transforms would carry the
//! site's translation through both and cancel it, rounding included. It is
//! composed instead from the placement chain the two share: both chains are
//! walked up through `IfcLocalPlacement.PlacementRelTo` to their first
//! common placement, and only the links below it are composed. An opening
//! placed relative to its host (as exporters write it) is then exactly its
//! own `RelativePlacement`; one placed beside it, relative to a storey or
//! site, is the host's links inverted and the opening's composed, all
//! metre-sized. The site's translation never enters.
//!
//! A chain is followed only through `IfcLocalPlacement` links: a grid or
//! linear placement (#357, #362, #363) ends the walk, and may itself be the
//! common placement. Nothing below it is then grid- or linear-dependent, and
//! the host's world transform, which does resolve those, is the one
//! `Instance` above the result.
//!
//! # The context frame, applied once
//!
//! The representation context's `WorldCoordinateSystem` (#357) belongs to
//! the host's world transform, and is applied once there. An opening's Body
//! named in a context whose frame differs from the host's would need it
//! inside the relative transform; that is not composed.
//!
//! # Fallback: the world frame, unchanged
//!
//! When any opening has no shared chain with its host -- an absolutely
//! placed opening, a placement that does not resolve as a local chain, a
//! context frame other than the host's -- the net body is lowered in world
//! coordinates exactly as before #388. Nothing is guessed: the relative
//! placement is either composed from what the file states or not used.

use std::collections::BTreeMap;

use axiolid_model::NodeId;
use ifc_model::{EntityId, Model};

use crate::constraint::local::LocalPlacement;
use crate::error::GeometryResult;
use crate::input::context::{representation_frame, representation_frame_resolved};
use crate::input::openings::Voiding;
use crate::input::product::Product;
use crate::input::representation::{select_product_representation, RepresentationPurpose};
use crate::lower::context::lower_product_representation_in;
use crate::lower::{lower_product_representation, LoweringSession};
use crate::transform::Transform;
use crate::units::UnitScale;

/// How deep a placement chain is walked before it is called malformed;
/// the resolver's own limit.
const MAX_CHAIN_DEPTH: usize = 64;

/// The frame a host's net body is lowered in.
#[derive(Debug, Clone)]
pub(super) enum NetFrame {
    /// The host's own frame: items lowered relative to it, the result placed
    /// once by `world`.
    Host {
        /// The host's world transform, metres, context frame included.
        world: Transform,
        /// Each opening's placement in the host's frame, metres.
        openings: BTreeMap<EntityId, Transform>,
    },
    /// World coordinates, as before #388: some opening shares no chain.
    World,
}

impl NetFrame {
    /// Decide the frame for `host` voided by `voidings`; `Ok(None)` when the
    /// host has no Body.
    ///
    /// Errors resolving the host's own world frame are returned as the gross
    /// lowering would return them. An opening whose relative placement does
    /// not compose selects [`NetFrame::World`], where the opening's own
    /// lowering reports whatever is wrong with it, as it always has.
    pub(super) fn for_host(
        session: &LoweringSession<'_>,
        host: EntityId,
        voidings: &[Voiding],
    ) -> GeometryResult<Option<Self>> {
        let (model, units) = (session.model(), session.units());
        let Some(world) = representation_frame_resolved(
            model,
            units,
            host,
            RepresentationPurpose::Body,
            session.linear_resolution(),
        )?
        else {
            return Ok(None);
        };
        let mut openings = BTreeMap::new();
        for voiding in voidings {
            if voiding.opening == host {
                // Refused by the caller, before anything is lowered for it.
                continue;
            }
            match select_product_representation(model, voiding.opening, RepresentationPurpose::Body)
            {
                // Nothing of it is lowered: taken as applied or refused, and
                // the frame never places anything.
                Ok(None) | Err(_) => continue,
                Ok(Some(_)) => {}
            }
            match opening_in_host(model, units, host, voiding.opening) {
                Some(relative) => {
                    openings.insert(voiding.opening, relative);
                }
                None => return Ok(Some(Self::World)),
            }
        }
        Ok(Some(Self::Host { world, openings }))
    }

    /// Lower the host's Body in this frame.
    pub(super) fn lower_host(
        &self,
        session: &mut LoweringSession<'_>,
        host: EntityId,
    ) -> GeometryResult<Option<NodeId>> {
        match self {
            Self::Host { .. } => lower_product_representation_in(
                session,
                host,
                RepresentationPurpose::Body,
                Transform::identity(),
            ),
            Self::World => lower_product_representation(session, host, RepresentationPurpose::Body),
        }
    }

    /// Lower an opening's Body in this frame.
    pub(super) fn lower_opening(
        &self,
        session: &mut LoweringSession<'_>,
        opening: EntityId,
    ) -> GeometryResult<Option<NodeId>> {
        match self {
            Self::Host { openings, .. } => match openings.get(&opening) {
                Some(relative) => lower_product_representation_in(
                    session,
                    opening,
                    RepresentationPurpose::Body,
                    *relative,
                ),
                // No Body was selected for it (see `for_host`), so this
                // selects none again or reports the same selection error;
                // no frame places anything.
                None => lower_product_representation_in(
                    session,
                    opening,
                    RepresentationPurpose::Body,
                    Transform::identity(),
                ),
            },
            Self::World => {
                lower_product_representation(session, opening, RepresentationPurpose::Body)
            }
        }
    }

    /// The host's world transform to put above a node lowered in this frame,
    /// or `None` when the node is already in world coordinates.
    pub(super) fn to_world(&self) -> Option<axiolid_core::Transform3> {
        match self {
            Self::Host { world, .. } if *world != Transform::identity() => Some(world.to_geom()),
            Self::Host { .. } | Self::World => None,
        }
    }
}

/// `opening`'s placement in `host`'s frame, in metres, composed along the
/// placement chain they share; `None` when they share none, or the two
/// Bodies are named in contexts with different frames.
fn opening_in_host(
    model: &Model,
    units: &UnitScale,
    host: EntityId,
    opening: EntityId,
) -> Option<Transform> {
    // The context frame cancels only when it is the same on both sides.
    if body_context_frame(model, units, host)? != body_context_frame(model, units, opening)? {
        return None;
    }
    let host_chain = local_chain(model, object_placement(model, host)?)?;
    let opening_chain = local_chain(model, object_placement(model, opening)?)?;
    let (below_opening, below_host) = opening_chain
        .iter()
        .enumerate()
        .find_map(|(i, id)| Some((i, host_chain.iter().position(|h| h == id)?)))?;
    let opening_from_common = compose_down(model, &opening_chain[..below_opening])?;
    let relative = if below_host == 0 {
        // The usual case: the opening is placed under the host's own
        // placement, and its links are the whole answer.
        opening_from_common
    } else {
        inverse(&compose_down(model, &host_chain[..below_host])?)?.compose(&opening_from_common)
    };
    Some(relative.to_metres(units))
}

/// The context frame `product`'s Body is placed in.
fn body_context_frame(model: &Model, units: &UnitScale, product: EntityId) -> Option<Transform> {
    let representation =
        select_product_representation(model, product, RepresentationPurpose::Body).ok()??;
    representation_frame(model, units, representation).ok()
}

/// `product`'s `ObjectPlacement`.
fn object_placement(model: &Model, product: EntityId) -> Option<EntityId> {
    Product::new(product, model.get(product)?).object_placement()
}

/// The placements from `start` up through `IfcLocalPlacement.PlacementRelTo`:
/// `[start, its parent, ...]`. Ends at a placement with no parent or one of
/// another kind, which is included. `None` on a cycle, an over-deep chain
/// or a dangling reference.
fn local_chain(model: &Model, start: EntityId) -> Option<Vec<EntityId>> {
    let mut chain = Vec::new();
    let mut current = start;
    loop {
        if chain.contains(&current) || chain.len() >= MAX_CHAIN_DEPTH {
            return None;
        }
        chain.push(current);
        let entity = model.get(current)?;
        if entity.type_name.as_ref() != "IFCLOCALPLACEMENT" {
            return Some(chain);
        }
        match LocalPlacement::new(current, entity).parent() {
            Some(parent) => current = parent,
            None => return Some(chain),
        }
    }
}

/// Compose the `RelativePlacement`s of `links` (a chain's lower part, the
/// placed end first), outermost first: the transform from the placement
/// above the last link down to the first. File units.
fn compose_down(model: &Model, links: &[EntityId]) -> Option<Transform> {
    let mut transform = Transform::identity();
    for &id in links.iter().rev() {
        let entity = model.get(id)?;
        if entity.type_name.as_ref() != "IFCLOCALPLACEMENT" {
            return None;
        }
        transform = transform.compose(
            &LocalPlacement::new(id, entity)
                .local_transform(model)
                .ok()?,
        );
    }
    Some(transform)
}

/// The inverse of an affine transform; `None` for a singular one.
fn inverse(transform: &Transform) -> Option<Transform> {
    let [a, b, c] = transform.basis;
    let rows = [cross(b, c), cross(c, a), cross(a, b)];
    let determinant = dot(a, rows[0]);
    if !determinant.is_finite() || determinant.abs() < 1e-12 {
        return None;
    }
    // Rows of the inverse are the cofactor rows over the determinant; the
    // basis is stored by columns.
    let basis = [0, 1, 2].map(|column| rows.map(|row| row[column] / determinant));
    let inverse = Transform {
        basis,
        origin: [0.0; 3],
    };
    let moved = inverse.apply_direction(transform.origin);
    Some(Transform {
        basis,
        origin: moved.map(|value| -value),
    })
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
    fn inverse_undoes_a_turned_translated_transform() {
        let (sin, cos) = 0.7f64.sin_cos();
        let transform = Transform {
            basis: [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]],
            origin: [13.475, -15.95, 4.3],
        };
        let product = inverse(&transform).expect("invertible").compose(&transform);
        assert!(product.is_identity(1e-14), "{product:?}");
        let product = transform.compose(&inverse(&transform).expect("invertible"));
        assert!(product.is_identity(1e-14), "{product:?}");
    }

    #[test]
    fn a_singular_transform_has_no_inverse() {
        let flat = Transform {
            basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
            origin: [0.0; 3],
        };
        assert!(inverse(&flat).is_none());
    }
}

//! Product placement and representation selection.
//!
//! # Why placement is composed here and not per item
//!
//! A representation item is authored in its product local space.
//! The product hangs off an IfcLocalPlacement chain that walks up
//! the spatial tree to the site. Lowering an item without that
//! chain puts every product at the world origin: the geometry is
//! individually correct and the building is a heap.
//!
//! # Units are converted once
//!
//! Placement coordinates are raw file units, so the resolver
//! composes the chain unconverted and this module converts the
//! composed result. Converting per link would raise the scale to
//! the power of the chain depth.

use axiolid_model::NodeId;
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
// The frame is kernel-free, so lowering and body description compose the
// same model space from one function.
use crate::input::context::representation_frame_resolved;
// Moved to `input::product`: it never needed the kernel. Re-exported so
// the pre-existing `lower::context::geometric_products` path still resolves.
pub use crate::input::product::geometric_products;
use crate::input::representation::{
    select_product_representation, Representation, RepresentationPurpose,
};
use crate::lower::dispatch::lower_representation_item;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

pub use crate::constraint::product_world_transform;
pub use crate::input::representation::select_shape_representation;

/// Lower every item of a product selected representation into one graph.
///
/// All items share one session, so a product whose Body holds several
/// solids yields one graph with one Collection root rather than N
/// disconnected graphs the caller has to merge.
pub fn lower_product_items(
    session: &mut LoweringSession<'_>,
    product: EntityId,
) -> GeometryResult<Option<NodeId>> {
    lower_product_representation(session, product, RepresentationPurpose::Body)
}

/// Select and lower a product representation for an explicit consumer path.
pub fn lower_product_representation(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    purpose: RepresentationPurpose,
) -> GeometryResult<Option<NodeId>> {
    // The session says how an `IfcLinearPlacement` resolves: from its cache,
    // or through the caller's evaluator (#353).
    let Some(world) = representation_frame_resolved(
        session.model(),
        session.units(),
        product,
        purpose,
        session.linear_resolution(),
    )?
    else {
        return Ok(None);
    };
    lower_product_representation_in(session, product, purpose, world)
}

/// Select and lower a product representation with every item placed by
/// `frame` instead of the product's world frame.
///
/// Net lowering (#388) lowers a host's Body in the host's own frame and
/// each opening's relative to it, then places the result once. `frame` is
/// in metres and already includes whatever context frame applies.
pub(crate) fn lower_product_representation_in(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    purpose: RepresentationPurpose,
    frame: Transform,
) -> GeometryResult<Option<NodeId>> {
    let Some(representation) = select_product_representation(session.model(), product, purpose)?
    else {
        return Ok(None);
    };

    let entity = session
        .model()
        .get(representation)
        .ok_or(GeometryError::MissingEntity {
            referrer: product,
            missing: representation,
        })?;
    let items = Representation::new(representation, entity).items()?;
    let mut roots = Vec::with_capacity(items.len());
    for item in items {
        roots.push(lower_representation_item(session, item, frame)?);
    }
    collect_roots(session, product, roots)
}

/// Select and lower a product representation, leaving out each item whose
/// STEP type name `leave_out` accepts.
///
/// `Ok(None)` when the product has no representation for `purpose`;
/// otherwise the root of the items lowered (`None` when every item was
/// left out) and the items left out, in authored order. Only the
/// representation's own items are tested: an item nested in a mapped
/// representation or a set is lowered, or refused, as usual. Every item
/// not left out lowers as [`lower_product_representation`] lowers it, and
/// its refusal still refuses the product. The curves reader is its only
/// caller (#512).
#[cfg(feature = "compile")]
pub(crate) fn lower_product_representation_leaving_out(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    purpose: RepresentationPurpose,
    leave_out: impl Fn(&str) -> bool,
) -> GeometryResult<Option<(Option<NodeId>, Vec<EntityId>)>> {
    let Some(frame) = representation_frame_resolved(
        session.model(),
        session.units(),
        product,
        purpose,
        session.linear_resolution(),
    )?
    else {
        return Ok(None);
    };
    let Some(representation) = select_product_representation(session.model(), product, purpose)?
    else {
        return Ok(None);
    };
    let entity = session
        .model()
        .get(representation)
        .ok_or(GeometryError::MissingEntity {
            referrer: product,
            missing: representation,
        })?;
    let items = Representation::new(representation, entity).items()?;
    let (mut roots, mut left_out) = (Vec::with_capacity(items.len()), Vec::new());
    for item in items {
        // A dangling item is lowered, so it is refused as usual.
        match session.model().get(item) {
            Some(entity) if leave_out(&entity.type_name) => left_out.push(item),
            _ => roots.push(lower_representation_item(session, item, frame)?),
        }
    }
    Ok(Some((collect_roots(session, product, roots)?, left_out)))
}

/// One root for the lowered items: none, the only one, or a collection.
fn collect_roots(
    session: &mut LoweringSession<'_>,
    product: EntityId,
    roots: Vec<NodeId>,
) -> GeometryResult<Option<NodeId>> {
    match roots.len() {
        0 => Ok(None),
        1 => Ok(Some(roots[0])),
        _ => Ok(Some(session.node_for(
            product,
            axiolid_model::GeometryNode::Collection(roots),
        )?)),
    }
}

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
// Moved to `input::product`: it never needed the kernel. Re-exported so
// the pre-existing `lower::context::geometric_products` path still resolves.
pub use crate::input::product::geometric_products;
use crate::input::representation::Representation;
use crate::lower::dispatch::lower_representation_item;
use crate::lower::session::LoweringSession;

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
    let world = product_world_transform(session.model(), session.units(), product)?;
    let Some(representation) = select_shape_representation(session.model(), product)? else {
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
        roots.push(lower_representation_item(session, item, world)?);
    }
    match roots.len() {
        0 => Ok(None),
        1 => Ok(Some(roots[0])),
        _ => Ok(Some(session.node_for(
            product,
            axiolid_model::GeometryNode::Collection(roots),
        )?)),
    }
}

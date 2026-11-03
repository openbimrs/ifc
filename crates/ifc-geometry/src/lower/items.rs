//! Per-item lowering of a product's Body: one root for each leaf item.
//!
//! [`lower_product_leaves`] walks the selected Body representation in
//! authored order, depth first, expanding every `IfcMappedItem` through its
//! `MappingSource.MappedRepresentation`. Every other item is a leaf, whatever
//! it holds (booleans, CSG and geometric sets included).
//!
//! # Same nodes as the whole Body
//!
//! A top-level leaf is lowered with the product's world frame, as
//! [`lower_product_representation`](super::context::lower_product_representation)
//! lowers it. A leaf inside a map is lowered once in the map's own frame (the
//! shared node) and wrapped in one `Instance` per enclosing mapped item, each
//! carrying `frame . MappingTarget . MappingOrigin` exactly as the whole-Body
//! lowering builds it. Only the `Collection` wrappers are left out, so a
//! single leaf compiles to the same mesh as a one-item Body.

use axiolid_model::{GeometryNode, Instance, NodeId};
use ifc_model::EntityId;

use crate::error::{GeometryError, GeometryResult};
use crate::input::context::representation_frame_resolved;
use crate::input::representation::{
    select_product_representation, Representation, RepresentationPurpose,
};
use crate::lower::dispatch::lower_representation_item;
use crate::lower::mapped::{mapped_occurrence, representation_items, KIND};
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

/// One leaf item of a Body and the root that lowers it.
pub(crate) struct LeafRoot {
    /// The representation item; never an `IfcMappedItem`.
    pub(crate) item: EntityId,
    /// The `IfcMappedItem`s it was reached through, outermost first.
    pub(crate) mapped_by: Vec<EntityId>,
    /// Root of the leaf in the session's graph, this occurrence placed.
    pub(crate) root: NodeId,
}

/// Lower each leaf item of `product`'s Body to its own root, in authored order.
///
/// `Ok(None)` when the product has no Body representation or frame.
pub(crate) fn lower_product_leaves(
    session: &mut LoweringSession<'_>,
    product: EntityId,
) -> GeometryResult<Option<Vec<LeafRoot>>> {
    let purpose = RepresentationPurpose::Body;
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

    let mut leaves = Vec::with_capacity(items.len());
    let mut stack = Vec::new();
    for item in items {
        visit(session, item, world, &mut stack, &mut leaves)?;
    }
    Ok(Some(leaves))
}

/// Visit `item`; `stack` holds the enclosing mapped items and their
/// placements, outermost first.
fn visit(
    session: &mut LoweringSession<'_>,
    item: EntityId,
    frame: Transform,
    stack: &mut Vec<(EntityId, Transform)>,
    leaves: &mut Vec<LeafRoot>,
) -> GeometryResult<()> {
    if session.type_name(item)? != "IFCMAPPEDITEM" {
        let mut node = if stack.is_empty() {
            lower_representation_item(session, item, frame)?
        } else {
            session.lower_operand(item, Transform::identity())?
        };
        // Innermost mapping first, so the outermost Instance ends on top.
        for &(mapped, placement) in stack.iter().rev() {
            node = session.node_for(
                mapped,
                GeometryNode::Instance(Instance {
                    source: node,
                    transform: placement.to_geom(),
                }),
            )?;
        }
        leaves.push(LeafRoot {
            item,
            mapped_by: stack.iter().map(|&(mapped, _)| mapped).collect(),
            root: node,
        });
        return Ok(());
    }

    session.enter(item, KIND)?;
    let result = visit_map(session, item, frame, stack, leaves);
    session.exit(item);
    result
}

fn visit_map(
    session: &mut LoweringSession<'_>,
    item: EntityId,
    frame: Transform,
    stack: &mut Vec<(EntityId, Transform)>,
    leaves: &mut Vec<LeafRoot>,
) -> GeometryResult<()> {
    let occurrence = mapped_occurrence(session, item, frame)?;
    session.enter(occurrence.representation, KIND)?;
    stack.push((item, occurrence.placement));
    let result = representation_items(session, occurrence.representation).and_then(|inner| {
        inner
            .into_iter()
            .try_for_each(|member| visit(session, member, Transform::identity(), stack, leaves))
    });
    stack.pop();
    session.exit(occurrence.representation);
    result
}

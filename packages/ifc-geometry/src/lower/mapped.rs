//! Mapped-item instancing: reuse one lowered subtree under many transforms.
//!
//! # Why this preserves instancing
//!
//! A mapped item is IFC's only first-class instancing mechanism. A furniture
//! family placed 400 times is authored once as an `IfcRepresentationMap` and
//! referenced by 400 `IfcMappedItem`s. Flattening that into 400 copies of the
//! geometry is the single most expensive mistake an importer can make, so the
//! lowered form keeps the shared subtree and emits one `Instance` per use.
//!
//! # Transform composition
//!
//! Three frames stack, outermost first:
//!
//! ```text
//!   world  o  MappingTarget  o  MappingOrigin
//! ```
//!
//! `MappingOrigin` locates the geometry inside the map's own space, and
//! `MappingTarget` places the instance. Getting the order backwards produces
//! geometry that is plausibly near the right place, which is far worse than an
//! obvious failure.

use axiolid_model::{GeometryNode, Instance, NodeId};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::mapped::{MappedItem, RepresentationMap};
use crate::resource::operator::operator_transform;
use crate::resource::placement::axis_placement_transform;
use crate::transform::Transform;

/// Family label for mapped-item memoization.
const MAPPED: &str = "mapped-item";

/// Family label for representation memoization.
const REPRESENTATION: &str = "representation";

/// Chain kind reported when a mapping graph closes on itself.
const KIND: &str = "mapped item";

/// Attribute index of `Items` on `IfcRepresentation`.
const REPRESENTATION_ITEMS: usize = 3;

/// Lower one `IfcMappedItem` into an `Instance` over its shared source.
///
/// The source representation is lowered once per world frame and memoized, so
/// repeated occurrences of one map share a subtree instead of duplicating it.
pub fn lower_mapped_item_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, MAPPED, world) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let node = lower_mapped_item_inner(session, id, world);
    session.exit(id);
    let node = node?;
    session.memoize(id, MAPPED, world, node);
    Ok(node)
}

fn lower_mapped_item_inner(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    world: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let item = MappedItem::new(id, entity);
    let source_ref = item.mapping_source()?;
    let target_ref = item.mapping_target()?;

    let map_entity = session.entity(id, source_ref)?;
    let map = RepresentationMap::new(source_ref, map_entity);
    let origin_ref = map.mapping_origin()?;
    let representation = map.mapped_representation()?;

    // MappingTarget places the instance; MappingOrigin locates geometry inside
    // the map's own space. Both are resolved before recursing so a malformed
    // operator fails fast against the item that referenced it.
    let target_entity = session.entity(id, target_ref)?;
    // Both frames carry file-unit coordinates; convert exactly once here.
    let target =
        operator_transform(session.model(), target_ref, target_entity)?.to_metres(session.units());
    let origin_entity = session.entity(source_ref, origin_ref)?;
    let origin = axis_placement_transform(session.model(), origin_ref, origin_entity)?
        .to_metres(session.units());

    // The source subtree is lowered in the map's own space. Keeping the world
    // frame OUT of the shared subtree is what lets many occurrences reuse it;
    // the per-occurrence placement rides on the Instance transform instead.
    let source = lower_representation(session, representation)?;

    let placement = world.compose(&target).compose(&origin);
    session.node_for(
        id,
        GeometryNode::Instance(Instance {
            source,
            transform: placement.to_geom(),
        }),
    )
}

/// Lower every item of an `IfcRepresentation` into one collection node.
///
/// A representation is an ordered set of items, so the neutral form is a
/// `Collection`. An empty item list is valid IFC and lowers to an empty
/// collection rather than an error.
pub fn lower_representation(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let frame = Transform::identity();
    if let Some(node) = session.memoized(id, REPRESENTATION, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let node = lower_representation_inner(session, id);
    session.exit(id);
    let node = node?;
    session.memoize(id, REPRESENTATION, frame, node);
    Ok(node)
}

fn lower_representation_inner(
    session: &mut LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    let slots = session.slots(id)?;
    let items = slots.req_ref_list(REPRESENTATION_ITEMS, "Items")?;

    let mut members = Vec::with_capacity(items.len());
    for item in items {
        // Items inside a map are lowered in the map's own space; the
        // occurrence transform lives on the Instance that wraps this
        // collection, never baked into the shared geometry.
        members.push(session.lower_operand(item, Transform::identity())?);
    }
    session.node_for(id, GeometryNode::Collection(members))
}

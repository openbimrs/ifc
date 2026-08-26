//! Reading the two ends of an objectified relationship.

use ifc_model::{EntityId, Model};

use super::slots::RelSlots;

/// Which objectified relationship a link came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipKind {
    /// `IfcRelAggregates` -- decomposition of a spatial structure or element.
    Aggregates,
    /// `IfcRelContainedInSpatialStructure` -- elements placed in a container.
    ContainedIn,
    /// `IfcRelNests` -- ordered decomposition.
    Nests,
}

/// One relationship instance, resolved to its ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The relationship entity itself.
    pub id: EntityId,
    /// Which relationship type this is.
    pub kind: RelationshipKind,
    /// The parent/owner end.
    pub relating: Option<EntityId>,
    /// The child/member ends, in file order.
    pub related: Vec<EntityId>,
}

/// Collect the references held in one attribute slot.
///
/// The relating end is normally a single reference and the related end a list,
/// but a malformed file can hold either shape in either slot, so both are read
/// the same way and interpreted by the caller.
pub(crate) fn refs_in_slot(model: &Model, id: EntityId, slot: usize) -> Vec<EntityId> {
    let Some(entity) = model.get(id) else {
        return Vec::new();
    };
    let Some(attribute) = entity.attribute(slot) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    attribute.for_each_ref(&mut |target| out.push(target));
    out
}

/// Read one relationship entity's ends, or `None` if it is not of this type.
pub(crate) fn read(
    model: &Model,
    id: EntityId,
    slots: RelSlots,
    kind: RelationshipKind,
) -> Option<Relationship> {
    let entity = model.get(id)?;
    if !entity.is_type(slots.type_name) {
        return None;
    }
    // A relating end naming several entities is malformed; take the first and
    // let the caller's validation report the rest rather than guessing.
    let relating = refs_in_slot(model, id, slots.relating).into_iter().next();
    Some(Relationship {
        id,
        kind,
        relating,
        related: refs_in_slot(model, id, slots.related),
    })
}

//! The pair-valued relationships: two elements, never a set.
//!
//! A few relationships connect exactly two elements --
//! `IfcRelConnectsElements` and its subtypes, `IfcRelInterferesElements`.
//! A set-shaped writer would accept a one-element or three-element list
//! for those and produce a record no reader can interpret, so they get
//! their own constructors taking two `EntityId`s. The refusal that matters
//! there is self-connection: an element connected to itself is a cycle the
//! connectivity reader in this crate will follow forever.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::authoring::{invalid, SpatialAuthoringResult};
use crate::relation::slots::{
    RelSlots, CONNECTS_ELEMENTS, CONNECTS_WITH_REALIZING, INTERFERES_ELEMENTS,
};

/// Refuse an empty `RealizingElements`: the subtype exists to name them.
pub(super) fn check_realizing(realizing: &[EntityId]) -> SpatialAuthoringResult<()> {
    if realizing.is_empty() {
        return Err(invalid(
            CONNECTS_WITH_REALIZING.type_name,
            "RealizingElements",
            "empty",
        ));
    }
    Ok(())
}

/// Refuse a relationship that connects an element to itself.
///
/// The connectivity reader walks these as a graph. A self-edge is
/// not a harmless oddity there: it is a cycle of length one.
pub(super) fn distinct(
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
) -> SpatialAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(rel.type_name, "GlobalId", global_id));
    }
    if relating == related {
        return Err(invalid(
            rel.type_name,
            "RelatedElement",
            "an element cannot connect to itself",
        ));
    }
    Ok(())
}

fn pair(
    tx: &mut Transaction,
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    width: usize,
) -> SpatialAuthoringResult<EntityId> {
    distinct(rel, global_id, relating, related)?;

    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[rel.relating] = Value::Ref(relating);
    attributes[rel.related] = Value::Ref(related);
    Ok(tx.create(Entity::new(rel.type_name, attributes)))
}

/// Stage an `IfcRelConnectsElements`: two elements physically joined.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`connect_elements_with_owner_history`](super::connect_elements_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an element connected to itself.
pub fn connect_elements(
    tx: &mut Transaction,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    pair(tx, CONNECTS_ELEMENTS, global_id, relating, related, 7)
}

/// Stage an `IfcRelConnectsWithRealizingElements`.
///
/// The realizing elements are what physically make the connection
/// -- a weld, a bolt, a bracket.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`connect_with_realizing_elements_with_owner_history`](super::connect_with_realizing_elements_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an element connected to itself,
/// and an empty realizing set: the subtype exists precisely to name
/// those elements, so omitting them makes it an
/// `IfcRelConnectsElements` wearing the wrong type name.
pub fn connect_with_realizing_elements(
    tx: &mut Transaction,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    realizing: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    check_realizing(realizing)?;
    let id = pair(tx, CONNECTS_WITH_REALIZING, global_id, relating, related, 8)?;
    tx.set_attribute(
        id,
        REALIZING_SLOT,
        Value::List(realizing.iter().copied().map(Value::Ref).collect()),
    );
    Ok(id)
}

/// `RealizingElements` on `IfcRelConnectsWithRealizingElements`.
const REALIZING_SLOT: usize = 7;

/// Stage an `IfcRelInterferesElements`: a detected clash.
///
/// `implied_order` is `ImpliedOrder`, an `IfcLogical` at slot 8. It
/// says whether the relating/related order carries meaning (which
/// element gives way). `None` writes UNKNOWN, which is the honest
/// value when a clash detector reports an overlap without deciding
/// precedence.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`interfere_elements_with_owner_history`](super::interfere_elements_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an element interfering with
/// itself.
pub fn interfere_elements(
    tx: &mut Transaction,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    implied_order: Option<bool>,
) -> SpatialAuthoringResult<EntityId> {
    let id = pair(tx, INTERFERES_ELEMENTS, global_id, relating, related, 10)?;
    tx.set_attribute(
        id,
        IMPLIED_ORDER_SLOT,
        implied_order.map_or(Value::LogicalUnknown, Value::Bool),
    );
    Ok(id)
}

/// `ImpliedOrder` on `IfcRelInterferesElements`.
const IMPLIED_ORDER_SLOT: usize = 8;

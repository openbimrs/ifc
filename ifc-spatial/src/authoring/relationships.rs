//! The remaining objectified relationships, and `IfcFacility`.
//!
//! # Set-valued and pair-valued relationships are not the same shape
//!
//! Most relationships here point one parent at a set of children, so
//! they go through [`super::relate`], which already refuses an empty
//! set and a parent listed among its own children.
//!
//! A few connect exactly two elements -- `IfcRelConnectsElements` and
//! its subtypes, `IfcRelInterferesElements`. A set-shaped writer would
//! accept a one-element or three-element list for those and produce a
//! record no reader can interpret, so they get their own constructors
//! taking two `EntityId`s. The refusal that matters there is
//! self-connection: an element connected to itself is a cycle the
//! connectivity reader in this crate will follow forever.
//!
//! # `IfcRelDefinesByObject` reverses the usual order
//!
//! Slot 4 is `RelatedObjects` and slot 5 is `RelatingObject`, the
//! opposite of `IfcRelAggregates`. The constant in `relation::slots`
//! already encodes that, which is exactly why these writers take
//! named `parent`/`children` arguments and resolve positions through
//! `RelSlots` rather than indexing literals.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::authoring::{invalid, SpatialAuthoringResult};
use crate::relation::slots::{
    RelSlots, ASSIGNS_TO_ACTOR, ASSIGNS_TO_GROUP_BY_FACTOR, ASSIGNS_TO_PROCESS, ASSIGNS_TO_PRODUCT,
    CONNECTS_ELEMENTS, CONNECTS_WITH_REALIZING, COVERS_ELEMENTS, COVERS_SPACES, DECLARES,
    DEFINES_BY_OBJECT, FLOW_CONTROL_ELEMENTS, INTERFERES_ELEMENTS, SERVICES_BUILDINGS,
};

/// Stage an `IfcRelCoversBldgElements`: finishes applied to an element.
///
/// Kept apart from [`cover_spaces`] because the same covering can do
/// both, and the two answer different questions: which finishes are on
/// this wall, versus which finishes bound this space.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty covering set, and the
/// element listed among its own coverings.
pub fn cover_elements(
    tx: &mut Transaction,
    global_id: &str,
    element: EntityId,
    coverings: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, COVERS_ELEMENTS, global_id, element, coverings)
}

/// Stage an `IfcRelCoversSpaces`: finishes bounding a space.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty covering set, and the space
/// listed among its own coverings.
pub fn cover_spaces(
    tx: &mut Transaction,
    global_id: &str,
    space: EntityId,
    coverings: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, COVERS_SPACES, global_id, space, coverings)
}

/// Stage an `IfcRelDeclares`: definitions declared in a context.
///
/// The context is an `IfcProject` or `IfcProjectLibrary`. This is how
/// type objects and property set templates enter a file without being
/// attached to any occurrence.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty definition set, and the
/// context listed among its own declarations.
pub fn declare(
    tx: &mut Transaction,
    global_id: &str,
    context: EntityId,
    definitions: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, DECLARES, global_id, context, definitions)
}

/// Stage an `IfcRelDefinesByObject`: occurrences defined by another object.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the
/// defining object listed among the objects it defines.
pub fn define_by_object(
    tx: &mut Transaction,
    global_id: &str,
    defining: EntityId,
    defined: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, DEFINES_BY_OBJECT, global_id, defining, defined)
}

/// Stage an `IfcRelServicesBuildings`: which spatial elements a system serves.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty building set, and the
/// system listed among the buildings it serves.
pub fn serve_buildings(
    tx: &mut Transaction,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, SERVICES_BUILDINGS, global_id, system, buildings)
}

/// Stage an `IfcRelFlowControlElements`: controls bound to a flow element.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty control set, and the flow
/// element listed among its own controls.
pub fn control_flow_element(
    tx: &mut Transaction,
    global_id: &str,
    flow_element: EntityId,
    controls: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, FLOW_CONTROL_ELEMENTS, global_id, flow_element, controls)
}

/// Stage an `IfcRelAssignsToActor`: objects assigned to an actor.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the actor
/// listed among the objects assigned to it.
pub fn assign_to_actor(
    tx: &mut Transaction,
    global_id: &str,
    actor: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ASSIGNS_TO_ACTOR, global_id, actor, objects)
}

/// Stage an `IfcRelAssignsToProduct`: objects assigned to a product.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the product
/// listed among the objects assigned to it.
pub fn assign_to_product(
    tx: &mut Transaction,
    global_id: &str,
    product: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ASSIGNS_TO_PRODUCT, global_id, product, objects)
}

/// Stage an `IfcRelAssignsToProcess`: objects assigned to a process.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the process
/// listed among the objects assigned to it.
pub fn assign_to_process(
    tx: &mut Transaction,
    global_id: &str,
    process: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ASSIGNS_TO_PROCESS, global_id, process, objects)
}

/// Stage an `IfcRelAssignsToGroupByFactor`.
///
/// The `factor` is an `IfcRatioMeasure` at slot 7, scaling each
/// member's contribution to the group.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty member set, the group
/// listed among its own members, and a non-finite factor.
pub fn assign_to_group_by_factor(
    tx: &mut Transaction,
    global_id: &str,
    group: EntityId,
    members: &[EntityId],
    factor: f64,
) -> SpatialAuthoringResult<EntityId> {
    if !factor.is_finite() {
        return Err(invalid(
            ASSIGNS_TO_GROUP_BY_FACTOR.type_name,
            "Factor",
            format!("expected a finite ratio, got {factor}"),
        ));
    }

    let id = super::relate(tx, ASSIGNS_TO_GROUP_BY_FACTOR, global_id, group, members)?;
    tx.set_attribute(id, FACTOR_SLOT, Value::Real(factor));
    Ok(id)
}

/// `Factor` on `IfcRelAssignsToGroupByFactor`, after the six
/// inherited `IfcRelAssigns` attributes and `RelatingGroup`.
const FACTOR_SLOT: usize = 7;

/// Refuse a relationship that connects an element to itself.
///
/// The connectivity reader walks these as a graph. A self-edge is
/// not a harmless oddity there: it is a cycle of length one.
fn distinct(
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
    if realizing.is_empty() {
        return Err(invalid(
            CONNECTS_WITH_REALIZING.type_name,
            "RealizingElements",
            "empty",
        ));
    }

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

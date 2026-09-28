//! The remaining objectified relationships, and `IfcFacility`.
//!
//! Most relationships here point one parent at a set of children, so
//! they go through [`super::relate`], which already refuses an empty
//! set and a parent listed among its own children. The pair-valued
//! ones live in `connections.rs`.
//!
//! # `IfcRelDefinesByObject` reverses the usual order
//!
//! Slot 4 is `RelatedObjects` and slot 5 is `RelatingObject`, the
//! opposite of `IfcRelAggregates`. The constant in `relation::slots`
//! already encodes that, which is exactly why these writers take
//! named `parent`/`children` arguments and resolve positions through
//! `RelSlots` rather than indexing literals.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::owned_relationships::relate_owned;
use crate::authoring::{invalid, SpatialAuthoringResult};
use crate::relation::slots::{
    ADHERES_TO_ELEMENT, ASSIGNS_TO_ACTOR, ASSIGNS_TO_GROUP_BY_FACTOR, ASSIGNS_TO_PROCESS,
    ASSIGNS_TO_PRODUCT, ASSIGNS_TO_RESOURCE, ASSOCIATES_PROFILE_DEF, COVERS_ELEMENTS,
    COVERS_SPACES, DECLARES, DEFINES_BY_OBJECT, FILLS_ELEMENT, FLOW_CONTROL_ELEMENTS, POSITIONS,
    PROJECTS_ELEMENT, SERVICES_BUILDINGS, VOIDS_ELEMENT,
};

/// Stage an `IfcRelCoversBldgElements`: finishes applied to an element.
///
/// Kept apart from [`cover_spaces`] because the same covering can do
/// both, and the two answer different questions: which finishes are on
/// this wall, versus which finishes bound this space.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`cover_elements_with_owner_history`](super::cover_elements_with_owner_history), which binds the model's declared
/// release.
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
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`cover_spaces_with_owner_history`](super::cover_spaces_with_owner_history), which binds the model's declared
/// release.
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
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`declare_with_owner_history`](super::declare_with_owner_history), which binds the model's declared
/// release.
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
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`define_by_object_with_owner_history`](super::define_by_object_with_owner_history), which binds the model's declared
/// release.
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
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`serve_buildings_with_owner_history`](super::serve_buildings_with_owner_history), which binds the model's declared
/// release.
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
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`control_flow_element_with_owner_history`](super::control_flow_element_with_owner_history), which binds the model's declared
/// release.
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
/// Bound to the model's declared release (#213): the record is laid out by
/// attribute name from that release's table, with all eight attributes
/// (`ActingRole` unset). `OwnerHistory` is left `$`, which IFC4 and IFC4X3
/// allow and IFC2X3 does not; in IFC2X3 use
/// [`assign_to_actor_with_owner_history`](super::assign_to_actor_with_owner_history).
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the actor
/// listed among the objects assigned to it. A header binding no single
/// verified release (`MultipleSchemas`, `UnsupportedSchema`) and an IFC2X3
/// model (`AuthoringRequired`) are refused. Nothing is staged on an error.
pub fn assign_to_actor(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    actor: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    relate_owned(
        tx,
        model,
        ASSIGNS_TO_ACTOR,
        global_id,
        actor,
        objects,
        Vec::new(),
        None,
    )
}

/// Stage an `IfcRelAssignsToProduct`: objects assigned to a product.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`assign_to_product_with_owner_history`](super::assign_to_product_with_owner_history), which binds the model's declared
/// release.
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
/// Bound to the model's declared release (#213): the record is laid out by
/// attribute name from that release's table, with all eight attributes
/// (`QuantityInProcess` unset). `OwnerHistory` is left `$`, which IFC4 and IFC4X3
/// allow and IFC2X3 does not; in IFC2X3 use
/// [`assign_to_process_with_owner_history`](super::assign_to_process_with_owner_history).
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the process
/// listed among the objects assigned to it. A header binding no single
/// verified release (`MultipleSchemas`, `UnsupportedSchema`) and an IFC2X3
/// model (`AuthoringRequired`) are refused. Nothing is staged on an error.
pub fn assign_to_process(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    process: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    relate_owned(
        tx,
        model,
        ASSIGNS_TO_PROCESS,
        global_id,
        process,
        objects,
        Vec::new(),
        None,
    )
}

/// Stage an `IfcRelAssignsToGroupByFactor`.
///
/// The `factor` is an `IfcRatioMeasure` at slot 7, scaling each
/// member's contribution to the group.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`assign_to_group_by_factor_with_owner_history`](super::assign_to_group_by_factor_with_owner_history), which binds the model's declared
/// release.
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
    check_factor(factor)?;
    let id = super::relate(tx, ASSIGNS_TO_GROUP_BY_FACTOR, global_id, group, members)?;
    tx.set_attribute(id, FACTOR_SLOT, Value::Real(factor));
    Ok(id)
}

/// Refuse a non-finite `IfcRatioMeasure` factor.
pub(super) fn check_factor(factor: f64) -> SpatialAuthoringResult<()> {
    if factor.is_finite() {
        return Ok(());
    }
    Err(invalid(
        ASSIGNS_TO_GROUP_BY_FACTOR.type_name,
        "Factor",
        format!("expected a finite ratio, got {factor}"),
    ))
}

/// `Factor` on `IfcRelAssignsToGroupByFactor`, after the six
/// inherited `IfcRelAssigns` attributes and `RelatingGroup`.
const FACTOR_SLOT: usize = 7;

/// Stage an `IfcRelVoidsElement`: an opening cut into an element.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`void_element_with_owner_history`](super::void_element_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an element voided by itself.
pub fn void_element(
    tx: &mut Transaction,
    global_id: &str,
    element: EntityId,
    opening: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    super::relate_one(tx, VOIDS_ELEMENT, global_id, element, opening)
}

/// Stage an `IfcRelFillsElement`: an element filling an opening.
///
/// Note the direction. The *opening* is the relating end here, the
/// inverse of [`void_element`]: a wall is voided by an opening, and
/// that opening is filled by a door.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`fill_element_with_owner_history`](super::fill_element_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an opening filled by itself.
pub fn fill_element(
    tx: &mut Transaction,
    global_id: &str,
    opening: EntityId,
    filling: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    super::relate_one(tx, FILLS_ELEMENT, global_id, opening, filling)
}

/// Stage an `IfcRelProjectsElement`: a feature added to an element.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`project_element_with_owner_history`](super::project_element_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an element projecting from itself.
pub fn project_element(
    tx: &mut Transaction,
    global_id: &str,
    element: EntityId,
    feature: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    super::relate_one(tx, PROJECTS_ELEMENT, global_id, element, feature)
}

/// Stage an `IfcRelAdheresToElement`: surface features bound to an element.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`adhere_to_element_with_owner_history`](super::adhere_to_element_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty feature set, and an element
/// listed among its own features.
pub fn adhere_to_element(
    tx: &mut Transaction,
    global_id: &str,
    element: EntityId,
    features: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ADHERES_TO_ELEMENT, global_id, element, features)
}

/// Stage an `IfcRelPositions`: products placed by a positioning element.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`position_products_with_owner_history`](super::position_products_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty product set, and the
/// positioning element listed among its own products, which the
/// schema's `NoSelfReference` rule forbids.
pub fn position_products(
    tx: &mut Transaction,
    global_id: &str,
    positioning: EntityId,
    products: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, POSITIONS, global_id, positioning, products)
}

/// Stage an `IfcRelAssignsToResource`: objects assigned to a resource.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`assign_to_resource_with_owner_history`](super::assign_to_resource_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the resource
/// listed among its own objects, which `NoSelfReference` forbids.
pub fn assign_to_resource(
    tx: &mut Transaction,
    global_id: &str,
    resource: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ASSIGNS_TO_RESOURCE, global_id, resource, objects)
}

/// Stage an `IfcRelAssociatesProfileDef`: a profile associated with objects.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`associate_profile_def_with_owner_history`](super::associate_profile_def_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty object set, and the profile
/// listed among its own objects.
pub fn associate_profile_def(
    tx: &mut Transaction,
    global_id: &str,
    profile: EntityId,
    objects: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    super::relate(tx, ASSOCIATES_PROFILE_DEF, global_id, profile, objects)
}

//! Transactional authoring of systems, ports and their connections.
//!
//! # Slot layouts come from the readers
//!
//! Each relationship here has its own idea of which end is which:
//! IfcRelAssignsToGroup names members at slot 4 and the group at 6,
//! IfcRelNests parent at 4 and children at 5, IfcRelConnectsPortToElement
//! port at 4 and element at 5. The reading side already encodes all of
//! that; authoring resolves the same constants so one cannot be
//! corrected without the other.
//!
//! # Releases and `OwnerHistory` (#202)
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from
//! IFC4 on:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! The writers without a model (`create_system`, `connect_ports`, ...)
//! cannot see the release. They write the IFC4/IFC4X3 layout with
//! `OwnerHistory` `$`, so their records are IFC4/IFC4X3 only. Each has a
//! `*_with_owner_history` variant that takes the model, binds its declared
//! release (`release.rs`), lays the record out by attribute name from that
//! release's table, and takes a caller-supplied `IfcOwnerHistory`, which
//! must exist (in the model or staged on the transaction) and be an
//! `IfcOwnerHistory`; one is never invented here. This follows
//! `ifc-material` (#77), `ifc-properties` (#191) and `ifc-classification`
//! (#194). In IFC4 and IFC4X3 a variant writes the plain writer's record
//! with the reference in the optional slot.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::connectivity::relation::slot as connects_slot;
use crate::port::definition::slot as port_slot;
use crate::system::group::slot as group_slot;
use crate::zone::spatial_group::slot as placement_slot;

mod distribution;
mod error;
mod owned;
mod release;
mod system_kind;

pub use distribution::{
    create_distribution_element, create_distribution_element_with_owner_history,
    create_spatial_zone, create_spatial_zone_with_owner_history, create_zone,
    create_zone_with_owner_history, DistributionElementKind, ElementAttributes,
};
pub use error::{SystemAuthoringError, SystemAuthoringResult};
pub use owned::{
    assign_to_group_with_owner_history, connect_port_to_element_with_owner_history,
    connect_ports_with_owner_history, contain_in_spatial_structure_with_owner_history,
    create_group_with_owner_history, create_port_with_owner_history,
    create_system_with_owner_history, nest_ports_with_owner_history,
    reference_in_spatial_structure_with_owner_history,
};
pub use system_kind::{
    create_classified_system, create_classified_system_with_owner_history, ClassifiedSystemDraft,
    SystemKind,
};

use error::invalid;

fn guid(entity: &'static str, global_id: &str) -> SystemAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    Ok(())
}

/// Refuse an empty related set, and the relating end listed in it.
fn check_related(
    entity: &'static str,
    attribute: &'static str,
    global_id: &str,
    relating: EntityId,
    related: &[EntityId],
    relating_word: &str,
) -> SystemAuthoringResult<()> {
    guid(entity, global_id)?;
    if related.is_empty() {
        return Err(invalid(entity, attribute, "empty"));
    }
    if related.contains(&relating) {
        return Err(invalid(
            entity,
            attribute,
            format!("contains the {relating_word}"),
        ));
    }
    Ok(())
}

/// Refuse a port connected to itself.
fn check_ports(
    global_id: &str,
    relating: EntityId,
    related: EntityId,
) -> SystemAuthoringResult<()> {
    guid("IFCRELCONNECTSPORTS", global_id)?;
    if relating == related {
        return Err(invalid(
            "IFCRELCONNECTSPORTS",
            "RelatedPort",
            "same as RelatingPort",
        ));
    }
    Ok(())
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// Stage an `IfcSystem`.
///
/// Writes the IFC4/IFC4X3 layout with `OwnerHistory` `$`, so the record
/// is IFC4/IFC4X3 only; use [`create_system_with_owner_history`] for a
/// release-bound record, which IFC2X3 needs.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_system(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
) -> SystemAuthoringResult<EntityId> {
    guid("IFCSYSTEM", global_id)?;
    let mut attributes = vec![Value::Null; 5];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = name.map_or(Value::Null, |t| Value::Text(t.into()));
    Ok(tx.create(Entity::new("IFCSYSTEM", attributes)))
}

/// Stage an `IfcDistributionPort`.
///
/// `flow_direction` is an `IfcFlowDirectionEnum` token: SOURCE, SINK
/// or SOURCEANDSINK. It is what the flow reader follows, so a port
/// without one is invisible to downstream/upstream queries.
///
/// Writes the IFC4/IFC4X3 ten-attribute layout with `OwnerHistory` `$`,
/// so the record is IFC4/IFC4X3 only; use
/// [`create_port_with_owner_history`] for IFC2X3, which declares eight.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_port(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    flow_direction: Option<&str>,
) -> SystemAuthoringResult<EntityId> {
    guid("IFCDISTRIBUTIONPORT", global_id)?;
    let mut attributes = vec![Value::Null; 10];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = name.map_or(Value::Null, |t| Value::Text(t.into()));
    attributes[7] = flow_direction.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new("IFCDISTRIBUTIONPORT", attributes)))
}

/// Stage an `IfcRelAssignsToGroup`: elements joined into a system.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`assign_to_group_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty member list, and the group
/// listed among its own members.
pub fn assign_to_group(
    tx: &mut Transaction,
    global_id: &str,
    group: EntityId,
    members: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    check_related(
        "IFCRELASSIGNSTOGROUP",
        "RelatedObjects",
        global_id,
        group,
        members,
        "group",
    )?;
    let width = group_slot::ASSIGNS_GROUP.max(group_slot::ASSIGNS_RELATED) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[group_slot::ASSIGNS_RELATED] = refs(members);
    attributes[group_slot::ASSIGNS_GROUP] = Value::Ref(group);
    Ok(tx.create(Entity::new("IFCRELASSIGNSTOGROUP", attributes)))
}

/// Stage an `IfcRelNests`: ports nested under the element owning them.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`nest_ports_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty child list, and a parent
/// nested under itself.
pub fn nest_ports(
    tx: &mut Transaction,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    check_related(
        "IFCRELNESTS",
        "RelatedObjects",
        global_id,
        parent,
        children,
        "parent",
    )?;
    let width = port_slot::NESTS_PARENT.max(port_slot::NESTS_CHILDREN) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[port_slot::NESTS_PARENT] = Value::Ref(parent);
    attributes[port_slot::NESTS_CHILDREN] = refs(children);
    Ok(tx.create(Entity::new("IFCRELNESTS", attributes)))
}

/// Stage an `IfcRelConnectsPortToElement`.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`connect_port_to_element_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn connect_port_to_element(
    tx: &mut Transaction,
    global_id: &str,
    port: EntityId,
    element: EntityId,
) -> SystemAuthoringResult<EntityId> {
    guid("IFCRELCONNECTSPORTTOELEMENT", global_id)?;
    let width = port_slot::PORT_TO_ELEMENT_PORT.max(port_slot::PORT_TO_ELEMENT_ELEMENT) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[port_slot::PORT_TO_ELEMENT_PORT] = Value::Ref(port);
    attributes[port_slot::PORT_TO_ELEMENT_ELEMENT] = Value::Ref(element);
    Ok(tx.create(Entity::new("IFCRELCONNECTSPORTTOELEMENT", attributes)))
}

/// Stage an `IfcRelConnectsPorts`: the link the flow graph walks.
///
/// `realizing` names the element that physically realises the
/// connection, such as the fitting between two segments.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`connect_ports_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId and a port connected to itself, which
/// is a self-loop the reachability walk would report as a component.
pub fn connect_ports(
    tx: &mut Transaction,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    realizing: Option<EntityId>,
) -> SystemAuthoringResult<EntityId> {
    check_ports(global_id, relating, related)?;
    let width = connects_slot::REALIZING + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[connects_slot::RELATING] = Value::Ref(relating);
    attributes[connects_slot::RELATED] = Value::Ref(related);
    attributes[connects_slot::REALIZING] = realizing.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCRELCONNECTSPORTS", attributes)))
}

/// Stage an `IfcRelContainedInSpatialStructure`.
///
/// Containment is exclusive: an element belongs to exactly one
/// structure. Use [`reference_in_spatial_structure`] for the
/// non-exclusive case, such as a duct crossing several storeys.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`contain_in_spatial_structure_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty element list, and the
/// structure listed among its own contents.
pub fn contain_in_spatial_structure(
    tx: &mut Transaction,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    place(
        tx,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        global_id,
        structure,
        elements,
    )
}

/// Stage an `IfcRelReferencedInSpatialStructure`.
///
/// The non-exclusive counterpart of containment.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`reference_in_spatial_structure_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty element list, and the
/// structure listed among its own references.
pub fn reference_in_spatial_structure(
    tx: &mut Transaction,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    place(
        tx,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        global_id,
        structure,
        elements,
    )
}

/// Both placement relationships share a layout: elements at 4,
/// structure at 5 -- the inverse of IfcRelAggregates.
fn place(
    tx: &mut Transaction,
    entity: &'static str,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    check_related(
        entity,
        "RelatedElements",
        global_id,
        structure,
        elements,
        "structure",
    )?;
    let width = placement_slot::RELATING_STRUCTURE.max(placement_slot::RELATED_ELEMENTS) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[placement_slot::RELATED_ELEMENTS] = refs(elements);
    attributes[placement_slot::RELATING_STRUCTURE] = Value::Ref(structure);
    Ok(tx.create(Entity::new(entity, attributes)))
}

/// Stage an `IfcGroup`: an arbitrary named collection.
///
/// A group is the supertype a system specialises. Where an
/// `IfcSystem` claims its members function together, a plain group
/// claims only that someone gathered them, so this writer is what to
/// reach for when no stronger statement is true.
///
/// Writes `OwnerHistory` `$`, so the record is IFC4/IFC4X3 only; use
/// [`create_group_with_owner_history`] for IFC2X3.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_group(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> SystemAuthoringResult<EntityId> {
    guid("IFCGROUP", global_id)?;
    let mut attributes = vec![Value::Null; 5];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = name.map_or(Value::Null, |t| Value::Text(t.into()));
    attributes[3] = description.map_or(Value::Null, |t| Value::Text(t.into()));
    Ok(tx.create(Entity::new("IFCGROUP", attributes)))
}

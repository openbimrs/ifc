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

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::connectivity::relation::slot as connects_slot;
use crate::port::definition::slot as port_slot;
use crate::system::group::slot as group_slot;
use crate::zone::spatial_group::slot as placement_slot;

mod distribution;
mod system_kind;

pub use distribution::{
    create_distribution_element, create_spatial_zone, create_zone, DistributionElementKind,
    ElementAttributes,
};
pub use system_kind::{create_classified_system, ClassifiedSystemDraft, SystemKind};

/// Why a systems record was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemAuthoringError {
    /// A value the schema constrains was not acceptable.
    Invalid {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute at fault.
        attribute: &'static str,
        /// What was supplied.
        value: String,
    },
}

impl std::fmt::Display for SystemAuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Self::Invalid {
            entity,
            attribute,
            value,
        } = self;
        write!(f, "{entity}.{attribute}: {value}")
    }
}

impl std::error::Error for SystemAuthoringError {}

/// Result of staging a systems record.
pub type SystemAuthoringResult<T> = Result<T, SystemAuthoringError>;

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> SystemAuthoringError {
    SystemAuthoringError::Invalid {
        entity,
        attribute,
        value: value.into(),
    }
}

fn guid(entity: &'static str, global_id: &str) -> SystemAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    Ok(())
}

/// Stage an `IfcSystem`.
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
    guid("IFCRELASSIGNSTOGROUP", global_id)?;
    if members.is_empty() {
        return Err(invalid("IFCRELASSIGNSTOGROUP", "RelatedObjects", "empty"));
    }
    if members.contains(&group) {
        return Err(invalid(
            "IFCRELASSIGNSTOGROUP",
            "RelatedObjects",
            "contains the group",
        ));
    }
    let width = group_slot::ASSIGNS_GROUP.max(group_slot::ASSIGNS_RELATED) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[group_slot::ASSIGNS_RELATED] =
        Value::List(members.iter().copied().map(Value::Ref).collect());
    attributes[group_slot::ASSIGNS_GROUP] = Value::Ref(group);
    Ok(tx.create(Entity::new("IFCRELASSIGNSTOGROUP", attributes)))
}

/// Stage an `IfcRelNests`: ports nested under the element owning them.
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
    guid("IFCRELNESTS", global_id)?;
    if children.is_empty() {
        return Err(invalid("IFCRELNESTS", "RelatedObjects", "empty"));
    }
    if children.contains(&parent) {
        return Err(invalid(
            "IFCRELNESTS",
            "RelatedObjects",
            "contains the parent",
        ));
    }
    let width = port_slot::NESTS_PARENT.max(port_slot::NESTS_CHILDREN) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[port_slot::NESTS_PARENT] = Value::Ref(parent);
    attributes[port_slot::NESTS_CHILDREN] =
        Value::List(children.iter().copied().map(Value::Ref).collect());
    Ok(tx.create(Entity::new("IFCRELNESTS", attributes)))
}

/// Stage an `IfcRelConnectsPortToElement`.
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
    guid("IFCRELCONNECTSPORTS", global_id)?;
    if relating == related {
        return Err(invalid(
            "IFCRELCONNECTSPORTS",
            "RelatedPort",
            "same as RelatingPort",
        ));
    }
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
    guid(entity, global_id)?;
    if elements.is_empty() {
        return Err(invalid(entity, "RelatedElements", "empty"));
    }
    if elements.contains(&structure) {
        return Err(invalid(entity, "RelatedElements", "contains the structure"));
    }
    let width = placement_slot::RELATING_STRUCTURE.max(placement_slot::RELATED_ELEMENTS) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[placement_slot::RELATED_ELEMENTS] =
        Value::List(elements.iter().copied().map(Value::Ref).collect());
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

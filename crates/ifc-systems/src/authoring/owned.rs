//! Release-bound variants of the systems writers, with a caller-supplied
//! `IfcOwnerHistory` (#202).
//!
//! Each binds the model's declared release, runs the plain writer's draft
//! checks, lays the record out by attribute name from the release's table
//! and sets `OwnerHistory`, which IFC2X3 requires. The attribute names are
//! the ones the readers' slot constants stand for; the unit test below pins
//! each name to its constant in all three bundled releases.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::release::bind;
use super::{check_ports, check_related, guid, refs, SystemAuthoringResult};

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}

/// [`create_system`](super::create_system) in `model`'s declared release,
/// with a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// The release is bound from `FILE_SCHEMA` (none binds IFC4). The owner
/// history is never invented: build it with `ifc-author`.
///
/// # Errors
///
/// Those of [`create_system`](super::create_system), and:
/// [`MultipleSchemas`](super::SystemAuthoringError::MultipleSchemas) or
/// [`UnsupportedSchema`](super::SystemAuthoringError::UnsupportedSchema) if
/// the model binds no single known release;
/// [`MissingReference`](super::SystemAuthoringError::MissingReference) if
/// `owner_history` is neither in the model nor staged;
/// [`WrongReferenceType`](super::SystemAuthoringError::WrongReferenceType)
/// if it is not an `IfcOwnerHistory`. Nothing is staged on an error.
pub fn create_system_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCSYSTEM";
    let release = bind(model)?;
    guid(ENTITY, global_id)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", text(name)),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`create_port`](super::create_port) in `model`'s declared release, with
/// a caller-supplied `IfcOwnerHistory`.
///
/// IFC2X3 declares eight attributes for `IfcDistributionPort`, IFC4 and
/// IFC4X3 ten; `FlowDirection` is the eighth in all three.
///
/// # Errors
///
/// Those of [`create_port`](super::create_port), the release and
/// owner-history refusals of [`create_system_with_owner_history`], and
/// [`AuthoringValueType`](super::SystemAuthoringError::AuthoringValueType)
/// for a token the release's `IfcFlowDirectionEnum` does not declare.
pub fn create_port_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    flow_direction: Option<&str>,
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCDISTRIBUTIONPORT";
    let release = bind(model)?;
    guid(ENTITY, global_id)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", text(name)),
        (
            "FlowDirection",
            flow_direction.map_or(Value::Null, |t| Value::Enum(t.into())),
        ),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`assign_to_group`](super::assign_to_group) in `model`'s declared
/// release, with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`assign_to_group`](super::assign_to_group), and the release
/// and owner-history refusals of [`create_system_with_owner_history`].
pub fn assign_to_group_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    group: EntityId,
    members: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELASSIGNSTOGROUP";
    let release = bind(model)?;
    check_related(ENTITY, "RelatedObjects", global_id, group, members, "group")?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatedObjects", refs(members)),
        ("RelatingGroup", Value::Ref(group)),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`nest_ports`](super::nest_ports) in `model`'s declared release, with a
/// caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`nest_ports`](super::nest_ports), and the release and
/// owner-history refusals of [`create_system_with_owner_history`].
pub fn nest_ports_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELNESTS";
    let release = bind(model)?;
    check_related(
        ENTITY,
        "RelatedObjects",
        global_id,
        parent,
        children,
        "parent",
    )?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingObject", Value::Ref(parent)),
        ("RelatedObjects", refs(children)),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`connect_port_to_element`](super::connect_port_to_element) in
/// `model`'s declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`connect_port_to_element`](super::connect_port_to_element),
/// and the release and owner-history refusals of
/// [`create_system_with_owner_history`].
pub fn connect_port_to_element_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    port: EntityId,
    element: EntityId,
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELCONNECTSPORTTOELEMENT";
    let release = bind(model)?;
    guid(ENTITY, global_id)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingPort", Value::Ref(port)),
        ("RelatedElement", Value::Ref(element)),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`connect_ports`](super::connect_ports) in `model`'s declared release,
/// with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`connect_ports`](super::connect_ports), and the release and
/// owner-history refusals of [`create_system_with_owner_history`].
pub fn connect_ports_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    realizing: Option<EntityId>,
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELCONNECTSPORTS";
    let release = bind(model)?;
    check_ports(global_id, relating, related)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingPort", Value::Ref(relating)),
        ("RelatedPort", Value::Ref(related)),
        (
            "RealizingElement",
            realizing.map_or(Value::Null, Value::Ref),
        ),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

/// [`contain_in_spatial_structure`](super::contain_in_spatial_structure) in
/// `model`'s declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of
/// [`contain_in_spatial_structure`](super::contain_in_spatial_structure),
/// and the release and owner-history refusals of
/// [`create_system_with_owner_history`].
pub fn contain_in_spatial_structure_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    place(
        tx,
        model,
        "IFCRELCONTAINEDINSPATIALSTRUCTURE",
        global_id,
        structure,
        elements,
        owner_history,
    )
}

/// [`reference_in_spatial_structure`](super::reference_in_spatial_structure)
/// in `model`'s declared release, with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of
/// [`reference_in_spatial_structure`](super::reference_in_spatial_structure),
/// and the release and owner-history refusals of
/// [`create_system_with_owner_history`].
pub fn reference_in_spatial_structure_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    place(
        tx,
        model,
        "IFCRELREFERENCEDINSPATIALSTRUCTURE",
        global_id,
        structure,
        elements,
        owner_history,
    )
}

fn place(
    tx: &mut Transaction,
    model: &Model,
    entity: &'static str,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    let release = bind(model)?;
    check_related(
        entity,
        "RelatedElements",
        global_id,
        structure,
        elements,
        "structure",
    )?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatedElements", refs(elements)),
        ("RelatingStructure", Value::Ref(structure)),
    ];
    release.stage(tx, model, entity, values, owner_history)
}

/// [`create_group`](super::create_group) in `model`'s declared release,
/// with a caller-supplied `IfcOwnerHistory`.
///
/// # Errors
///
/// Those of [`create_group`](super::create_group), and the release and
/// owner-history refusals of [`create_system_with_owner_history`].
pub fn create_group_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCGROUP";
    let release = bind(model)?;
    guid(ENTITY, global_id)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", text(name)),
        ("Description", text(description)),
    ];
    release.stage(tx, model, ENTITY, values, owner_history)
}

#[cfg(test)]
mod tests {
    use ifc_schema::{for_version, SchemaVersion};

    use crate::authoring::distribution::{distribution_slot, spatial_zone_slot, zone_slot};
    use crate::connectivity::relation::slot as connects_slot;
    use crate::port::definition::slot as port_slot;
    use crate::system::group::slot as group_slot;
    use crate::zone::spatial_group::slot as placement_slot;

    /// Every attribute name the variants write is the one the reader's (and
    /// the plain writer's) slot constant stands for, in all three releases
    /// that declare the entity.
    #[test]
    fn names_agree_with_the_slot_constants() {
        let pinned: &[(&str, &str, usize)] = &[
            (
                "IFCRELASSIGNSTOGROUP",
                "RelatedObjects",
                group_slot::ASSIGNS_RELATED,
            ),
            (
                "IFCRELASSIGNSTOGROUP",
                "RelatingGroup",
                group_slot::ASSIGNS_GROUP,
            ),
            ("IFCRELNESTS", "RelatingObject", port_slot::NESTS_PARENT),
            ("IFCRELNESTS", "RelatedObjects", port_slot::NESTS_CHILDREN),
            (
                "IFCRELCONNECTSPORTTOELEMENT",
                "RelatingPort",
                port_slot::PORT_TO_ELEMENT_PORT,
            ),
            (
                "IFCRELCONNECTSPORTTOELEMENT",
                "RelatedElement",
                port_slot::PORT_TO_ELEMENT_ELEMENT,
            ),
            (
                "IFCDISTRIBUTIONPORT",
                "FlowDirection",
                port_slot::FLOW_DIRECTION,
            ),
            (
                "IFCRELCONNECTSPORTS",
                "RelatingPort",
                connects_slot::RELATING,
            ),
            ("IFCRELCONNECTSPORTS", "RelatedPort", connects_slot::RELATED),
            (
                "IFCRELCONNECTSPORTS",
                "RealizingElement",
                connects_slot::REALIZING,
            ),
            (
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                "RelatedElements",
                placement_slot::RELATED_ELEMENTS,
            ),
            (
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                "RelatingStructure",
                placement_slot::RELATING_STRUCTURE,
            ),
            (
                "IFCRELREFERENCEDINSPATIALSTRUCTURE",
                "RelatedElements",
                placement_slot::RELATED_ELEMENTS,
            ),
            (
                "IFCRELREFERENCEDINSPATIALSTRUCTURE",
                "RelatingStructure",
                placement_slot::RELATING_STRUCTURE,
            ),
            ("IFCFLOWSEGMENT", "Name", distribution_slot::NAME),
            (
                "IFCFLOWSEGMENT",
                "Description",
                distribution_slot::DESCRIPTION,
            ),
            (
                "IFCFLOWSEGMENT",
                "ObjectPlacement",
                distribution_slot::OBJECT_PLACEMENT,
            ),
            (
                "IFCFLOWSEGMENT",
                "Representation",
                distribution_slot::REPRESENTATION,
            ),
            ("IFCFLOWSEGMENT", "Tag", distribution_slot::TAG),
            ("IFCZONE", "Name", zone_slot::NAME),
            ("IFCZONE", "LongName", zone_slot::LONG_NAME),
            ("IFCSPATIALZONE", "LongName", spatial_zone_slot::LONG_NAME),
            (
                "IFCSPATIALZONE",
                "PredefinedType",
                spatial_zone_slot::PREDEFINED_TYPE,
            ),
        ];
        let mut checked = 0;
        for version in [
            SchemaVersion::Ifc2x3,
            SchemaVersion::Ifc4,
            SchemaVersion::Ifc4x3,
        ] {
            let schema = for_version(version).unwrap();
            for (entity, name, slot) in pinned {
                let names = schema.attribute_names(entity);
                if schema.entity(entity).is_none() {
                    continue;
                }
                if let Some(found) = names.iter().position(|n| n == name) {
                    assert_eq!(found, *slot, "{version:?} {entity}.{name}");
                    checked += 1;
                } else {
                    // Only IFC2X3 may lack one: IfcZone.LongName.
                    assert_eq!(
                        (version, *entity, *name),
                        (SchemaVersion::Ifc2x3, "IFCZONE", "LongName")
                    );
                }
            }
        }
        assert!(checked >= 60, "the pin table matched {checked} names");
    }
}

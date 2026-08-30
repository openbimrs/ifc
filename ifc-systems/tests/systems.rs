//! `ifc-systems` SYS-ROOT: systems and membership.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_systems::{
    ports, systems, Attachment, ConnectionGraph, FlowDirection, NetworkGraph, SystemAnomaly,
};

/// Build a model stating one distribution system with two members.
/// The committed fixture: a real STEP file, not a synthetic model.
fn fixture() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-systems/synthetic_systems.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn model_with_system() -> Model {
    let mut model = Model::new();
    let seg = EntityId(1);
    model.insert(seg, Entity::new("IfcFlowSegment", vec![]));
    let fitting = EntityId(2);
    model.insert(fitting, Entity::new("IfcFlowFitting", vec![]));
    let system = EntityId(3);
    model.insert(
        system,
        Entity::new(
            "IfcDistributionSystem",
            vec![
                Value::Text("guid".into()),
                Value::Null,
                Value::Text("Heating".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Enum("HEATING".into()),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IfcRelAssignsToGroup",
            vec![
                Value::Text("relguid".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(seg), Value::Ref(fitting)]),
                Value::Null,
                Value::Ref(system),
            ],
        ),
    );
    model
}

/// A subtype system is found, not just an exact `IfcSystem`.
///
/// `Model::ids_of_type` is an exact index, so a file whose only system is an
/// `IfcDistributionSystem` -- the overwhelmingly common case -- would report
/// no systems at all if the crate asked for `IFCSYSTEM` directly.
#[test]
fn a_distribution_system_is_found_as_a_system() {
    let (found, anomalies) = systems(&model_with_system());
    assert_eq!(found.len(), 1, "the distribution system must be found");
    assert_eq!(found[0].type_name, "IFCDISTRIBUTIONSYSTEM");
    assert_eq!(found[0].name.as_deref(), Some("Heating"));
    assert!(anomalies.is_empty(), "clean file: {anomalies:?}");
}

/// Membership is read from slot 6, not slot 5.
///
/// `IfcRelAssignsToGroup` carries `RelatedObjectsType` at slot 5 and the group
/// at 6, unlike every other relationship in this crate. Reading 5 yields an
/// enumeration and the membership silently disappears.
#[test]
fn members_are_read_from_the_relating_group_slot() {
    let (found, _) = systems(&model_with_system());
    assert_eq!(found[0].members, vec![EntityId(1), EntityId(2)]);
}

/// A system with nothing assigned to it is still reported.
///
/// Membership is stated by a separate relationship, so a declared but empty
/// system is a normal state in a partially-authored file. Discovering systems
/// by walking memberships would drop it and understate the model.
#[test]
fn a_system_with_no_members_is_still_a_system() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IfcDistributionSystem",
            vec![
                Value::Text("guid".into()),
                Value::Null,
                Value::Text("Empty".into()),
            ],
        ),
    );
    let (found, anomalies) = systems(&model);
    assert_eq!(found.len(), 1);
    assert!(found[0].members.is_empty());
    assert!(anomalies.is_empty());
}

/// A membership naming an absent entity is reported, not silently dropped.
#[test]
fn a_dangling_member_is_reported() {
    let mut model = Model::new();
    let system = EntityId(1);
    model.insert(
        system,
        Entity::new(
            "IfcDistributionSystem",
            vec![Value::Text("guid".into()), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IfcRelAssignsToGroup",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(99))]),
                Value::Null,
                Value::Ref(system),
            ],
        ),
    );
    let (found, anomalies) = systems(&model);
    assert!(
        found[0].members.is_empty(),
        "the absent member is not a member"
    );
    assert_eq!(
        anomalies,
        vec![SystemAnomaly::Dangling {
            relation: EntityId(2),
            missing: EntityId(99),
        }]
    );
}

/// Assignment to a group that is not a system is recorded, not made a member.
///
/// `IfcRelAssignsToGroup` is shared with every group kind, so the relationship
/// alone does not imply a system. Note that `IfcZone` does NOT belong in this
/// test: in IFC4 its supertype chain is IfcZone -> IfcSystem -> IfcGroup, so a
/// zone genuinely IS a system and treating it as an intruder would be wrong.
/// `IfcInventory` is a sibling group that is not a system.
#[test]
fn assignment_to_a_non_system_group_is_not_a_system_membership() {
    let mut model = Model::new();
    let inventory = EntityId(1);
    model.insert(
        inventory,
        Entity::new(
            "IfcInventory",
            vec![Value::Text("guid".into()), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IfcRelAssignsToGroup",
            vec![
                Value::Text("rel".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(3))]),
                Value::Null,
                Value::Ref(inventory),
            ],
        ),
    );
    let (found, anomalies) = systems(&model);
    assert!(found.is_empty(), "an IfcInventory is not an IfcSystem");
    assert_eq!(
        anomalies,
        vec![SystemAnomaly::NotASystem {
            relation: EntityId(2),
            group: inventory,
            type_name: "IFCINVENTORY".to_string(),
        }]
    );
}

/// A zone IS a system in IFC4, and must be discovered as one.
///
/// This is the inverse of the test above and exists to stop a future
/// "zones are not systems" simplification: the schema says otherwise, and a
/// zone dropped from system discovery is a silently missing part of the model.
#[test]
fn a_zone_is_discovered_because_the_schema_makes_it_a_system() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IfcZone",
            vec![
                Value::Text("guid".into()),
                Value::Null,
                Value::Text("Fire compartment".into()),
            ],
        ),
    );
    let (found, _) = systems(&model);
    assert_eq!(found.len(), 1, "IfcZone -> IfcSystem in the IFC4 schema");
    assert_eq!(found[0].name.as_deref(), Some("Fire compartment"));
}

/// The committed fixture is read end to end, not just hand-built models.
///
/// A synthetic in-memory model proves the slot arithmetic; it does not prove
/// the crate survives a real STEP file with owner histories, subtypes and
/// unrelated entities interleaved.
#[test]
fn the_committed_fixture_reads_its_systems() {
    let model = fixture();
    let (found, anomalies) = systems(&model);

    // Two distribution systems plus one zone: the zone counts because
    // IfcZone -> IfcSystem in IFC4.
    let mut names: Vec<_> = found.iter().filter_map(|s| s.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["Fire compartment", "Heating", "Ventilation"]);

    let heating = found
        .iter()
        .find(|s| s.name.as_deref() == Some("Heating"))
        .expect("heating system");
    assert_eq!(
        heating.members.len(),
        4,
        "two pipes, a fitting and a terminal"
    );

    // The IfcInventory assignment must be reported as a non-system, not
    // silently dropped and not counted as a system membership.
    assert!(
        anomalies.iter().any(|a| matches!(
            a,
            SystemAnomaly::NotASystem { type_name, .. } if type_name == "IFCINVENTORY"
        )),
        "the inventory assignment must be reported: {anomalies:?}"
    );
}

/// Systems come back in a deterministic order, ascending by id.
///
/// Discovery walks a type histogram, whose iteration order is not the file's
/// order. Without an explicit sort the result would vary between runs and
/// between files that differ only in entity numbering, so any caller
/// comparing two exports would see spurious differences.
#[test]
fn systems_are_returned_in_ascending_id_order() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-systems/synthetic_systems.ifc");
    let model = ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses");
    let (found, _) = systems(&model);
    let ids: Vec<_> = found.iter().map(|s| s.id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "system order must be deterministic");
    assert!(ids.len() >= 3, "fixture states three systems");
}

// ---- SYS-PORT --------------------------------------------------------------

/// Ports attached by BOTH mechanisms are found.
///
/// IFC4 nests ports under their element; IFC2x3 used a dedicated
/// relationship, and real exporters still emit it. A reader that knows only
/// one form silently loses every port attached the other way -- and the loss
/// is invisible, because the ports still exist as entities.
#[test]
fn ports_attached_by_either_mechanism_resolve_to_their_element() {
    let model = fixture();
    let (ports, _) = ports(&model);

    let nested = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("seg0-in"))
        .expect("nested port");
    assert_eq!(nested.attachment, Some(Attachment::Nests));
    assert!(nested.element.is_some(), "a nested port has an element");

    let legacy = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("seg1-in"))
        .expect("legacy port");
    assert_eq!(legacy.attachment, Some(Attachment::ConnectsPortToElement));
    assert!(legacy.element.is_some(), "a legacy port has an element");

    assert_ne!(
        nested.element, legacy.element,
        "the two ports belong to different segments"
    );
}

/// An abstract supertype is never a file's literal type.
///
/// No entity is ever an IFCPORT: every port is an IfcDistributionPort. Asking
/// the exact-type index for IFCPORT returns nothing at all.
#[test]
fn ports_are_found_by_ancestry_not_by_exact_type() {
    let model = fixture();
    assert!(
        model.ids_of_type("IFCPORT").is_empty(),
        "IfcPort is abstract; nothing is literally an IFCPORT"
    );
    let (ports, _) = ports(&model);
    assert_eq!(ports.len(), 11, "every distribution port is found");
}

/// An unattached port is reported with no element, not dropped.
#[test]
fn an_unattached_port_keeps_its_place_with_no_element() {
    let model = fixture();
    let (ports, _) = ports(&model);
    let orphan = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("orphan"))
        .expect("the unattached port is still listed");
    assert_eq!(orphan.element, None);
    assert_eq!(orphan.attachment, None);
}

/// Flow direction is read from slot 7 and parsed exactly.
#[test]
fn flow_direction_is_read_from_its_own_slot() {
    let model = fixture();
    let (ports, _) = ports(&model);
    let by = |name: &str| {
        ports
            .iter()
            .find(|p| p.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("fixture has {name}"))
            .flow
    };
    assert_eq!(by("seg0-in"), FlowDirection::Sink);
    assert_eq!(by("seg0-out"), FlowDirection::Source);
    assert_eq!(by("fit-c"), FlowDirection::SourceAndSink);
    assert_eq!(by("orphan"), FlowDirection::NotDefined);
}

// ---- SYS-CONN --------------------------------------------------------------

/// The connection graph is undirected.
///
/// RelatingPort/RelatedPort record authoring order, not flow. If the graph
/// were directed by schema order, half of every network would be unreachable
/// depending only on which end the exporter wrote first.
#[test]
fn the_connection_graph_is_undirected() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let connection = graph
        .connections()
        .first()
        .expect("the fixture states connections");

    assert!(
        graph
            .neighbours(connection.relating)
            .contains(&connection.related),
        "forward edge"
    );
    assert!(
        graph
            .neighbours(connection.related)
            .contains(&connection.relating),
        "reverse edge: order is authoring order, not direction"
    );
}

/// RealizingElement is read from slot 6 when present.
#[test]
fn a_connection_keeps_its_realizing_element() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let realized = graph
        .connections()
        .iter()
        .filter(|c| c.realizing.is_some())
        .count();
    assert_eq!(
        realized, 1,
        "exactly one connection names a realizing element"
    );
}

/// Stated connections alone do NOT connect a chain.
///
/// This is the trap. IfcRelConnectsPorts joins one element's port to
/// another's; it never joins an element's own inlet to its own outlet. So a
/// physically continuous run of pipe has a connection graph of isolated
/// PAIRS, and "what is downstream" answered from it returns almost nothing.
#[test]
fn stated_connections_alone_leave_a_chain_in_pieces() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);

    let start = ports_list
        .iter()
        .find(|p| p.name.as_deref() == Some("seg0-in"))
        .expect("fixture port")
        .id;

    // seg0-in is named by exactly one connection, to fit-c.
    assert_eq!(
        graph.reachable_from(start).len(),
        2,
        "the raw connection graph reaches only the directly-named partner"
    );
    assert_eq!(
        graph.components().len(),
        5,
        "five stated connections, five disconnected pairs"
    );
}

/// Adding through-element edges connects the network and terminates on a ring.
///
/// The fixture deliberately closes a loop (fit-c back to seg0-in). Cycles are
/// NORMAL in distribution systems -- ring mains, recirculating circuits -- so
/// traversal must terminate by design. Without a visited set this hangs
/// rather than failing, which is why it is pinned.
#[test]
fn the_network_graph_connects_the_chain_and_terminates_on_a_ring() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let network = NetworkGraph::build(&graph, &ports_list);

    let start = ports_list
        .iter()
        .find(|p| p.name.as_deref() == Some("seg0-in"))
        .expect("fixture port")
        .id;

    // seg0(2) + seg1(2) + fitting(3) + terminal(1) = 8 ports in the loop.
    assert_eq!(
        network.reachable_from(start).len(),
        8,
        "the whole heating run is reachable, each port exactly once"
    );
}

/// A separate system forms its own component.
#[test]
fn a_disconnected_system_is_its_own_component() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let network = NetworkGraph::build(&graph, &ports_list);

    let mut sizes: Vec<_> = network.components().iter().map(Vec::len).collect();
    sizes.sort_unstable();
    assert_eq!(
        sizes,
        vec![2, 8],
        "ventilation's two ports stay separate from the heating run"
    );
}

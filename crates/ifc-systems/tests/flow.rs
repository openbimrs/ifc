//! `ifc-systems`: flow roles, zones, spatial placement and flow queries.

mod common;

use common::fixture;
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_systems::{
    ports, role_inconsistencies, spatial_placements, zones, ConnectionGraph, ElementRole,
    FlowNetwork, RoleInconsistency, SystemAnomaly,
};

// ---- SYS-FLOW ------------------------------------------------------------

/// A role is read from the element type, by ancestry.
///
/// `IfcFlowSegment` and friends have subtypes (`IfcPipeSegment`, ...), so an
/// exact type match would classify a real file's elements as unknown.
#[test]
fn element_roles_come_from_schema_ancestry() {
    let model = fixture();
    let (ports, _) = ports(&model);

    let mut roles = std::collections::BTreeMap::new();
    for port in &ports {
        if let Some(element) = port.element {
            if let Some(role) = ElementRole::of(&model, element) {
                roles.insert(element, role);
            }
        }
    }

    let segments = roles
        .values()
        .filter(|r| **r == ElementRole::Segment)
        .count();
    assert_eq!(
        segments, 6,
        "three pipes, the backwards one, and the dead-end pair"
    );
    assert_eq!(
        roles
            .values()
            .filter(|r| **r == ElementRole::Terminal)
            .count(),
        1,
        "the radiator"
    );
    assert_eq!(
        roles
            .values()
            .filter(|r| **r == ElementRole::MovingDevice)
            .count(),
        1,
        "the pump"
    );
}

/// An element with no way in (or out) is reported.
///
/// A segment with two SOURCE ports cannot receive anything. Nothing in the
/// schema catches this: element type and port direction are stated
/// independently, so only a cross-check finds it.
#[test]
fn an_element_with_no_inlet_is_reported() {
    let mut model = Model::default();
    let mut add = |id: u64, ty: &str, attrs: Vec<Value>| {
        model.insert(
            EntityId(id),
            Entity {
                type_name: ty.into(),
                attributes: attrs,
            },
        );
    };
    let port = |name: &str, dir: &str| {
        vec![
            Value::Text("g".into()),
            Value::Null,
            Value::Text(name.into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Enum(dir.into()),
        ]
    };
    add(1, "IFCFLOWSEGMENT", vec![Value::Text("S".into())]);
    add(2, "IFCDISTRIBUTIONPORT", port("a", "SOURCE"));
    add(3, "IFCDISTRIBUTIONPORT", port("b", "SOURCE"));
    add(
        4,
        "IFCRELNESTS",
        vec![
            Value::Text("g".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(EntityId(1)),
            Value::List(vec![Value::Ref(EntityId(2)), Value::Ref(EntityId(3))]),
        ],
    );

    let (ports, _) = ports(&model);
    let found = role_inconsistencies(&model, &ports);
    assert!(
        found.iter().any(|i| matches!(
            i,
            RoleInconsistency::NoPath { element, has_inlet, .. }
                if *element == EntityId(1) && !has_inlet
        )),
        "two SOURCE ports means nothing can enter: {found:?}"
    );
}

// ---- SYS-ZONE ------------------------------------------------------------

/// WR1 is enforced: a non-spatial zone member is excluded AND reported.
///
/// IfcOpenShell's own validator passes this fixture, so the check is not
/// redundant with schema validation -- WHERE rules are not enforced there.
#[test]
fn a_zone_member_that_wr1_forbids_is_excluded_and_reported() {
    let model = fixture();
    let (found, anomalies) = zones(&model);

    let zone = found.first().expect("one zone");
    assert_eq!(zone.members.len(), 2, "only the two spaces are members");

    assert!(
        anomalies.iter().any(|a| matches!(
            a,
            SystemAnomaly::ZoneMemberNotSpatial { type_name, .. }
                if type_name.eq_ignore_ascii_case("IfcFlowTerminal")
        )),
        "the flow terminal violates WR1: {anomalies:?}"
    );
}

/// Containment and referencing are kept apart.
///
/// seg2 passes THROUGH two spaces without being contained by either. Merging
/// the two relationships would report it as contained twice, or lose the
/// references entirely.
#[test]
fn containment_and_referencing_are_not_merged() {
    let model = fixture();
    let (placements, anomalies) = spatial_placements(&model);
    assert!(
        anomalies.is_empty(),
        "fixture is well-formed: {anomalies:?}"
    );

    let passing_through = placements
        .values()
        .find(|p| p.contained_in.is_none() && p.referenced_in.len() == 2)
        .expect("seg2 is referenced by two spaces and contained by none");
    assert_eq!(passing_through.referenced_in.len(), 2);

    let contained = placements
        .values()
        .filter(|p| p.contained_in.is_some())
        .count();
    assert_eq!(contained, 4, "three heating elements plus the radiator");
}

// ---- SYS-QUERY -----------------------------------------------------------

/// Downstream follows flow; upstream does not return the same set.
///
/// This is the whole point of orienting the graph: undirected reachability
/// would give an identical answer in both directions.
#[test]
fn downstream_and_upstream_differ() {
    let model = fixture();
    let (ports, _) = ports(&model);
    let (graph, _) = ConnectionGraph::build(&model);
    let network = FlowNetwork::build(&graph, &ports);

    // The first pipe in the heating chain.
    let seg0 = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("seg0-out"))
        .and_then(|p| p.element)
        .expect("seg0 has an outlet");

    let down = network.downstream_of(seg0);
    let up = network.upstream_of(seg0);
    assert_ne!(
        down.elements, up.elements,
        "an oriented network must distinguish the two"
    );
    assert!(
        !down.elements.is_empty(),
        "seg0 feeds the rest of the chain"
    );
}

/// A query crossing an unstated port says that it did.
///
/// The pump's ports are NOTDEFINED. Reaching it is right -- the file connects
/// it -- but the caller must be able to tell the answer rests on treating
/// silence as bidirectional.
#[test]
fn an_unstated_direction_is_flagged_not_hidden() {
    let model = fixture();
    let (ports, _) = ports(&model);
    let (graph, _) = ConnectionGraph::build(&model);
    let network = FlowNetwork::build(&graph, &ports);

    let seg0 = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("seg0-out"))
        .and_then(|p| p.element)
        .expect("seg0");

    let pump = ports
        .iter()
        .find(|p| p.name.as_deref() == Some("pump-a"))
        .and_then(|p| p.element)
        .expect("pump");

    let down = network.downstream_of(seg0);
    assert!(
        down.elements.contains(&pump),
        "the pump is connected and must be reachable"
    );
    assert!(
        down.used_undirected,
        "reaching it relied on an unstated direction"
    );
}

/// A directed query terminates on a ring main.
#[test]
fn a_directed_query_terminates_on_a_loop() {
    let model = fixture();
    let (ports, _) = ports(&model);
    let (graph, _) = ConnectionGraph::build(&model);
    let network = FlowNetwork::build(&graph, &ports);

    for port in &ports {
        if let Some(element) = port.element {
            // Any start must return, loop or not.
            let _ = network.downstream_of(element);
            let _ = network.upstream_of(element);
        }
    }
}

/// A segment whose ports both emit is reported as having no path.
///
/// The schema states element type and port direction independently, so this
/// file is valid: nothing rejects a pipe that cannot be flowed through.
/// Catching it needs the cross-check, which is the whole point of SYS-FLOW.
#[test]
fn an_element_that_cannot_pass_flow_is_reported() {
    let model = fixture();
    let (ports_list, _) = ports(&model);
    let found = role_inconsistencies(&model, &ports_list);

    let no_path = found
        .iter()
        .filter_map(|i| match i {
            RoleInconsistency::NoPath {
                element,
                has_inlet,
                has_outlet,
                ..
            } => Some((*element, *has_inlet, *has_outlet)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        no_path.iter().any(|(_, inlet, outlet)| !*inlet && *outlet),
        "the two-SOURCE segment has an outlet but no inlet: {no_path:?}"
    );
}

/// Flow cannot leave through a SINK.
///
/// Orientation is the entire contract of a directed query. If a sink were
/// walkable outward, downstream would wander back up the supply and every
/// answer would silently include the wrong half of the network.
#[test]
fn a_sink_does_not_emit_downstream() {
    let mut model = Model::default();
    let mut add = |id: u64, ty: &str, attrs: Vec<Value>| {
        model.insert(
            EntityId(id),
            Entity {
                type_name: ty.into(),
                attributes: attrs,
            },
        );
    };
    // element 1 owns a-out(SOURCE); element 2 owns b-in(SINK) and b-back(SINK).
    add(1, "IFCFLOWSEGMENT", vec![]);
    add(2, "IFCFLOWSEGMENT", vec![]);
    let port = |name: &str, flow: &str| {
        vec![
            Value::Text(name.into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Enum(flow.into()),
        ]
    };
    add(10, "IFCDISTRIBUTIONPORT", port("a-out", "SOURCE"));
    add(11, "IFCDISTRIBUTIONPORT", port("b-in", "SINK"));
    add(12, "IFCDISTRIBUTIONPORT", port("b-back", "SINK"));
    add(
        20,
        "IFCRELNESTS",
        vec![
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Ref(EntityId(1)),
            Value::List(vec![Value::Ref(EntityId(10))]),
        ],
    );
    add(
        21,
        "IFCRELNESTS",
        vec![
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Ref(EntityId(2)),
            Value::List(vec![Value::Ref(EntityId(11)), Value::Ref(EntityId(12))]),
        ],
    );
    add(
        30,
        "IFCRELCONNECTSPORTS",
        vec![
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Text("".into()),
            Value::Ref(EntityId(10)),
            Value::Ref(EntityId(11)),
        ],
    );

    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let flow = FlowNetwork::build(&graph, &ports_list);

    // Downstream of element 1 reaches element 2 -- flow enters its SINK.
    let down = flow.downstream_of(EntityId(1));
    assert!(
        down.elements.contains(&EntityId(2)),
        "flow enters element 2 through its sink"
    );

    // But nothing flows OUT of element 2: both its ports are sinks.
    let from_two = flow.downstream_of(EntityId(2));
    assert!(
        from_two.elements.is_empty(),
        "an element with only sinks emits nothing downstream: {:?}",
        from_two.elements
    );
}

/// An element contained by two structures is reported, not silently reduced.
///
/// `ContainedInStructure` is `SET [0:1]`. IfcOpenShell rejects the breach, so
/// it cannot live in a committed fixture -- but exporters still write it, and
/// last-writer-wins would hide a real modelling error.
#[test]
fn an_element_contained_twice_is_reported() {
    let mut model = Model::default();
    let mut add = |id: u64, ty: &str, attrs: Vec<Value>| {
        model.insert(
            EntityId(id),
            Entity {
                type_name: ty.into(),
                attributes: attrs,
            },
        );
    };
    add(1, "IFCFLOWSEGMENT", vec![]);
    add(2, "IFCSPACE", vec![]);
    add(3, "IFCSPACE", vec![]);
    for (id, structure) in [(10u64, 2u64), (11, 3)] {
        add(
            id,
            "IFCRELCONTAINEDINSPATIALSTRUCTURE",
            vec![
                Value::Text("".into()),
                Value::Text("".into()),
                Value::Text("".into()),
                Value::Text("".into()),
                Value::List(vec![Value::Ref(EntityId(1))]),
                Value::Ref(EntityId(structure)),
            ],
        );
    }

    let (placements, anomalies) = spatial_placements(&model);

    assert!(
        anomalies
            .iter()
            .any(|a| matches!(a, SystemAnomaly::ContainedTwice { element, .. } if *element == EntityId(1))),
        "the second containment must be reported: {anomalies:?}"
    );
    // The first claim still stands: one broken relationship does not
    // invalidate the rest of the placement map.
    assert_eq!(
        placements.get(&EntityId(1)).and_then(|p| p.contained_in),
        Some(EntityId(2)),
        "the first stated container is kept"
    );
}

/// Flow does not run backwards along the heating chain.
///
/// The chain is seg0 -> seg1 -> fitting -> terminal, oriented by SOURCE/SINK
/// ports. Asking what the TERMINAL feeds must return nothing: its only port
/// is a SINK. If exit-through-a-sink were permitted the query would walk the
/// chain in reverse and report the whole run, so this pins direction itself
/// rather than merely pinning an empty result.
#[test]
fn flow_does_not_run_backwards_through_a_sink() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let flow = FlowNetwork::build(&graph, &ports_list);

    let terminal = ports_list
        .iter()
        .find(|p| p.name.as_deref() == Some("term-in"))
        .and_then(|p| p.element)
        .expect("the radiator owns term-in");

    assert!(
        flow.downstream_of(terminal).elements.is_empty(),
        "a radiator whose only port is a SINK feeds nothing"
    );

    // Upstream from the same element DOES reach the run, which proves the
    // emptiness above is orientation and not a disconnected fixture.
    assert!(
        !flow.upstream_of(terminal).elements.is_empty(),
        "the radiator is fed by the chain"
    );
}

/// A sink cannot emit, even when the port it faces would accept.
///
/// Every WELL-FORMED connection pairs a SOURCE with a SINK, which makes the
/// exit and entry rules redundant: either alone blocks a reverse walk. They
/// disagree only on a malformed SINK <-> SINK pair, which real exporters do
/// emit. The fixture states one so the exit rule is observable behaviour
/// rather than unreachable defensive code.
#[test]
fn a_sink_cannot_emit_even_towards_an_accepting_port() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let flow = FlowNetwork::build(&graph, &ports_list);

    let owner = |port: &str| {
        ports_list
            .iter()
            .find(|p| p.name.as_deref() == Some(port))
            .and_then(|p| p.element)
            .expect("fixture port is attached")
    };

    let dead = owner("dead-a");
    let other = owner("dead-b");

    // dead-a is a SINK facing dead-b, also a SINK. Entry would be permitted
    // (a sink accepts), so only the exit rule stops the walk.
    assert!(
        flow.downstream_of(dead).elements.is_empty(),
        "a SINK port cannot emit, so nothing is downstream"
    );
    assert!(
        !flow.upstream_of(dead).elements.contains(&other),
        "and the mirrored rule holds walking upstream"
    );
}
/// The pump branch is reachable downstream, and only downstream.
///
/// The heating loop is a RING, so almost everything is mutually reachable
/// and a containment check proves little there. The pump hangs off the
/// fitting through fit-d (SOURCE) -> pump-a, a one-way spur: it is
/// downstream of the fitting and the fitting is NOT downstream of it.
/// A relaxed exit rule makes that spur bidirectional, which this catches.
#[test]
fn a_one_way_spur_is_reachable_in_one_direction_only() {
    let model = fixture();
    let (graph, _) = ConnectionGraph::build(&model);
    let (ports_list, _) = ports(&model);
    let flow = FlowNetwork::build(&graph, &ports_list);

    let owner = |port: &str| {
        ports_list
            .iter()
            .find(|p| p.name.as_deref() == Some(port))
            .and_then(|p| p.element)
            .expect("fixture port is attached")
    };

    let fitting = owner("fit-d");
    let pump = owner("pump-a");

    assert!(
        flow.downstream_of(fitting).elements.contains(&pump),
        "the fitting feeds the pump through fit-d"
    );
    assert!(
        !flow.downstream_of(pump).elements.contains(&fitting),
        "the spur is one-way: the pump does not feed the fitting back"
    );
}

//! An authored distribution system, read back by the systems readers.
//!
//! Asserting through ports(), systems() and the connection graph
//! proves the authored slots are the ones those readers resolve.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_systems::{
    assign_to_group, connect_port_to_element, connect_ports, contain_in_spatial_structure,
    create_port, create_system, nest_ports, ports, reference_in_spatial_structure,
    spatial_placements, systems, ConnectionGraph,
};

/// A pipe segment to carry flow.
fn segment(tx: &mut Transaction, guid: &str) -> EntityId {
    let mut attributes = vec![Value::Null; 8];
    attributes[0] = Value::Text(guid.into());
    tx.create(Entity::new("IFCFLOWSEGMENT", attributes))
}

/// A whole system authored, then read back by the crate's readers.
#[test]
fn an_authored_system_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let system =
        create_system(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", Some("Chilled water")).expect("system");
    let a = segment(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu");
    let b = segment(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu");
    let out = create_port(
        &mut tx,
        "3aBcDeFgHiJkLmNoPqRsTu",
        Some("Out"),
        Some("SOURCE"),
    )
    .expect("out port");
    let inp =
        create_port(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", Some("In"), Some("SINK")).expect("in port");

    nest_ports(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", a, &[out]).expect("nest a");
    nest_ports(&mut tx, "06BcDeFgHiJkLmNoPqRsTu", b, &[inp]).expect("nest b");
    connect_port_to_element(&mut tx, "07BcDeFgHiJkLmNoPqRsTu", out, a).expect("attach a");
    connect_port_to_element(&mut tx, "08BcDeFgHiJkLmNoPqRsTu", inp, b).expect("attach b");
    connect_ports(&mut tx, "09BcDeFgHiJkLmNoPqRsTu", out, inp, None).expect("connect");
    assign_to_group(&mut tx, "0ABcDeFgHiJkLmNoPqRsTu", system, &[a, b]).expect("assign");
    tx.commit(&mut model).expect("commit");

    let (found, anomalies) = systems(&model);
    assert!(
        anomalies.is_empty(),
        "authored system is clean: {anomalies:?}"
    );
    assert_eq!(found.len(), 1);

    let (read_ports, port_anomalies) = ports(&model);
    assert!(
        port_anomalies.is_empty(),
        "ports are clean: {port_anomalies:?}"
    );
    assert_eq!(read_ports.len(), 2);

    // The two segments are one connected component through their ports.
    let graph = ConnectionGraph::build(&model).0;
    assert_eq!(graph.neighbours(out), vec![inp]);
    assert_eq!(graph.reachable_from(out), vec![out, inp]);
}

/// Records that parse but describe no system are refused.
#[test]
fn meaningless_connections_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let g = "1aBcDeFgHiJkLmNoPqRsTu";
    let sys = create_system(&mut tx, g, Some("S")).expect("system");
    let p = create_port(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", None, Some("SOURCE")).expect("port");

    // A malformed GlobalId leaves the record unreferenceable.
    assert!(create_system(&mut tx, "nope", None).is_err());
    assert!(create_port(&mut tx, "nope", None, None).is_err());
    assert!(connect_ports(&mut tx, "nope", p, p, None).is_err());

    // A group with no members is not a system.
    assert!(assign_to_group(&mut tx, g, sys, &[]).is_err());
    assert!(nest_ports(&mut tx, g, p, &[]).is_err());

    // Self-membership and self-nesting are cycles the readers walk.
    assert!(assign_to_group(&mut tx, g, sys, &[sys]).is_err());
    assert!(nest_ports(&mut tx, g, p, &[p]).is_err());

    // A port connected to itself is a self-loop in the flow graph.
    assert!(connect_ports(&mut tx, g, p, p, None).is_err());
}

/// Each relationship writes its ends to the slots the schema names.
///
/// The readers here are forgiving enough to answer correctly even when
/// the two ends are transposed, so a round-trip assertion does not
/// prove placement. These check the raw slots: IfcRelAssignsToGroup
/// members at 4 and group at 6, IfcRelNests parent at 4 and children
/// at 5, IfcRelConnectsPortToElement port at 4 and element at 5, and
/// IfcDistributionPort.FlowDirection at 7.
#[test]
fn every_relationship_writes_its_schema_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let sys = create_system(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", None).expect("sys");
    let el = segment(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu");
    let port = create_port(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", None, Some("SINK")).expect("port");
    let grp = assign_to_group(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", sys, &[el]).expect("grp");
    let nst = nest_ports(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", el, &[port]).expect("nst");
    let att = connect_port_to_element(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", port, el).expect("att");
    tx.commit(&mut model).expect("commit");

    let slot = |id, n: usize| model.get(id).unwrap().attributes[n].clone();
    let list = |id| Value::List(vec![Value::Ref(id)]);

    assert_eq!(slot(grp, 4), list(el), "assigns: members at 4");
    assert_eq!(slot(grp, 6), Value::Ref(sys), "assigns: group at 6");
    assert_eq!(slot(nst, 4), Value::Ref(el), "nests: parent at 4");
    assert_eq!(slot(nst, 5), list(port), "nests: children at 5");
    assert_eq!(slot(att, 4), Value::Ref(port), "attach: port at 4");
    assert_eq!(slot(att, 5), Value::Ref(el), "attach: element at 5");
    assert_eq!(
        slot(port, 7),
        Value::Enum("SINK".into()),
        "port: flow direction at 7"
    );
}

/// Containment and reference read back as the distinct kinds they are.
///
/// Containment is exclusive and reference is not; the placement reader
/// keeps them in separate fields, so authoring one as the other is a
/// silent change of meaning rather than a parse error.
#[test]
fn placement_distinguishes_containment_from_reference() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let storey = segment(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu");
    let other = segment(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu");
    let duct = segment(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu");
    let con = contain_in_spatial_structure(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", storey, &[duct])
        .expect("contained");
    reference_in_spatial_structure(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", other, &[duct])
        .expect("referenced");
    tx.commit(&mut model).expect("commit");

    let (placements, anomalies) = spatial_placements(&model);
    assert!(
        anomalies.is_empty(),
        "authored placement is clean: {anomalies:?}"
    );
    let placement = &placements[&duct];
    assert_eq!(placement.contained_in, Some(storey));
    assert_eq!(placement.referenced_in, vec![other]);

    // Elements at 4, structure at 5: the inverse of IfcRelAggregates.
    let slot = |id, n: usize| model.get(id).unwrap().attributes[n].clone();
    assert_eq!(slot(con, 4), Value::List(vec![Value::Ref(duct)]));
    assert_eq!(slot(con, 5), Value::Ref(storey));

    let mut tx = Transaction::new(&model);
    let g = "05BcDeFgHiJkLmNoPqRsTu";
    assert!(contain_in_spatial_structure(&mut tx, g, storey, &[]).is_err());
    assert!(reference_in_spatial_structure(&mut tx, g, storey, &[storey]).is_err());
}

//! Systems authoring bound to the declared release (#202).
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is required in IFC2X3
//! TC1 and `OPTIONAL` in IFC4 ADD2 TC1 and IFC4X3 ADD2. IFC2X3 declares
//! `IfcDistributionPort` with eight attributes (IFC4: ten) and `IfcZone`
//! with five (no `LongName`), and no `IfcSpatialZone`, `IfcBuildingSystem`,
//! `IfcDistributionSystem` or `IfcDistributionCircuit`; `IfcBuiltSystem` is
//! IFC4X3 only. Every relationship these writers stage has the same
//! attribute names in all three.

use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_schema::for_version;
use ifc_step::StepCodec;
use ifc_systems::authoring::{
    create_distribution_element_with_owner_history, create_spatial_zone_with_owner_history,
    create_zone_with_owner_history, DistributionElementKind, ElementAttributes,
};
use ifc_systems::{
    assign_to_group_with_owner_history, connect_port_to_element_with_owner_history,
    connect_ports_with_owner_history, contain_in_spatial_structure_with_owner_history,
    create_classified_system_with_owner_history, create_group_with_owner_history,
    create_port_with_owner_history, create_system_with_owner_history,
    nest_ports_with_owner_history, ports, reference_in_spatial_structure_with_owner_history,
    systems, zones, ClassifiedSystemDraft, ConnectionGraph, SchemaVersion, SystemAnomaly,
    SystemKind,
};

const OWNER: EntityId = EntityId(5);
const STOREY: EntityId = EntityId(20);
const SPACE: EntityId = EntityId(21);

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`) with its actors, a storey (`#20`) and a space
/// (`#21`), in `schema`.
fn base(schema: &str, version: SchemaVersion) -> Model {
    // IFC2X3 requires IfcSpace.InteriorOrExteriorSpace; IFC4 replaced it by
    // an optional PredefinedType at the same position.
    let interior = if version == SchemaVersion::Ifc2x3 {
        ".INTERNAL."
    } else {
        "$"
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #20=IFCBUILDINGSTOREY('3YvctVUKr0kugbFTf53O00',#5,'L1',$,$,$,$,$,.ELEMENT.,$);\n\
         #21=IFCSPACE('3YvctVUKr0kugbFTf53O01',#5,'R1',$,$,$,$,$,.ELEMENT.,{interior},$);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    format!("2YvctVUKr0kugbFTf53O0{}", DIGITS[n] as char)
}

/// What the round trip authored, by role.
struct Authored {
    system: EntityId,
    zone: EntityId,
    segments: [EntityId; 2],
    ports: [EntityId; 2],
    written: Vec<EntityId>,
}

/// Author with the variants everything `version` can hold.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Authored {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let g = guid;
    let mut tx = Transaction::new(model);
    let m = &*model;
    let system =
        create_system_with_owner_history(&mut tx, m, &g(1), Some("Heating"), OWNER).unwrap();
    let group = create_group_with_owner_history(&mut tx, m, &g(2), Some("Lot"), Some("All"), OWNER)
        .unwrap();
    let long_name = ifc4.then_some("Thermal zone A");
    let zone = create_zone_with_owner_history(&mut tx, m, &g(3), Some("A"), None, long_name, OWNER)
        .unwrap();
    let element = |name| ElementAttributes {
        name: Some(name),
        tag: Some("T"),
        ..ElementAttributes::default()
    };
    let segment = DistributionElementKind::FlowSegment;
    let fitting = DistributionElementKind::FlowFitting;
    let s1 = create_distribution_element_with_owner_history(
        &mut tx,
        m,
        segment,
        &g(4),
        element("Pipe 1"),
        OWNER,
    )
    .unwrap();
    let s2 = create_distribution_element_with_owner_history(
        &mut tx,
        m,
        segment,
        &g(5),
        element("Pipe 2"),
        OWNER,
    )
    .unwrap();
    let bend = create_distribution_element_with_owner_history(
        &mut tx,
        m,
        fitting,
        &g(6),
        element("Bend"),
        OWNER,
    )
    .unwrap();
    let p1 = create_port_with_owner_history(&mut tx, m, &g(7), Some("Out"), Some("SOURCE"), OWNER)
        .unwrap();
    let p2 =
        create_port_with_owner_history(&mut tx, m, &g(8), Some("In"), Some("SINK"), OWNER).unwrap();
    let mut written = vec![system, group, zone, s1, s2, bend, p1, p2];
    if ifc4 {
        // IFC4 attaches ports by nesting.
        written.push(nest_ports_with_owner_history(&mut tx, m, &g(9), s1, &[p1], OWNER).unwrap());
        written.push(nest_ports_with_owner_history(&mut tx, m, &g(10), s2, &[p2], OWNER).unwrap());
    } else {
        // IFC2X3 has no port nesting; it connects a port to its element.
        written.push(
            connect_port_to_element_with_owner_history(&mut tx, m, &g(9), p1, s1, OWNER).unwrap(),
        );
        written.push(
            connect_port_to_element_with_owner_history(&mut tx, m, &g(10), p2, s2, OWNER).unwrap(),
        );
    }
    written.extend([
        connect_ports_with_owner_history(&mut tx, m, &g(11), p1, p2, Some(bend), OWNER).unwrap(),
        assign_to_group_with_owner_history(&mut tx, m, &g(12), system, &[s1, s2], OWNER).unwrap(),
        assign_to_group_with_owner_history(&mut tx, m, &g(13), zone, &[SPACE], OWNER).unwrap(),
        contain_in_spatial_structure_with_owner_history(
            &mut tx,
            m,
            &g(14),
            STOREY,
            &[s1, s2, bend],
            OWNER,
        )
        .unwrap(),
        reference_in_spatial_structure_with_owner_history(&mut tx, m, &g(15), SPACE, &[s1], OWNER)
            .unwrap(),
    ]);
    if ifc4 {
        let thermal = create_spatial_zone_with_owner_history(
            &mut tx,
            m,
            &g(16),
            element("Thermal"),
            Some("Thermal zone"),
            Some("THERMAL"),
            None,
            OWNER,
        )
        .unwrap();
        let heating = ClassifiedSystemDraft::new()
            .predefined_type("HEATING")
            .long_name("Heating water");
        let distribution = create_classified_system_with_owner_history(
            &mut tx,
            m,
            SystemKind::DistributionSystem,
            &g(17),
            Some("HW"),
            heating,
            OWNER,
        )
        .unwrap();
        let (kind, token) = if version == SchemaVersion::Ifc4x3 {
            (SystemKind::Built, "FOUNDATION")
        } else {
            (SystemKind::Building, "FOUNDATION")
        };
        let built = ClassifiedSystemDraft::new().predefined_type(token);
        let built = create_classified_system_with_owner_history(
            &mut tx,
            m,
            kind,
            &g(18),
            None,
            built,
            OWNER,
        )
        .unwrap();
        written.extend([thermal, distribution, built]);
    }
    tx.commit(model).expect("commit");
    Authored {
        system,
        zone,
        segments: [s1, s2],
        ports: [p1, p2],
        written,
    }
}

/// Each release round-trips: written, re-read with `ifc-step`, laid out by
/// name from that release's table with the owner history set, and read back
/// through the crate's views.
#[test]
fn systems_round_trip_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let authored = author(&mut model, version);
        let table = for_version(version).unwrap();
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for id in &authored.written {
            let record = back.get(*id).expect("read back");
            let names = table.attribute_names(&record.type_name);
            assert_eq!(record.attributes.len(), names.len(), "{schema} {record:?}");
            assert_eq!(names[1], "OwnerHistory");
            assert_eq!(
                record.attributes[1],
                Value::Ref(OWNER),
                "{schema} {record:?}"
            );
        }
        let [p1, p2] = authored.ports;
        let [s1, s2] = authored.segments;
        let port_arity = if version == SchemaVersion::Ifc2x3 {
            8
        } else {
            10
        };
        assert_eq!(back.get(p1).unwrap().attributes.len(), port_arity);
        assert_eq!(
            back.get(p1).unwrap().attributes[7],
            Value::Enum("SOURCE".into())
        );
        let zone_arity = if version == SchemaVersion::Ifc2x3 {
            5
        } else {
            6
        };
        assert_eq!(
            back.get(authored.zone).unwrap().attributes.len(),
            zone_arity
        );

        let (found, anomalies) = systems(&back);
        // IFC2X3 `IfcZone` is an `IfcGroup`, not an `IfcSystem` (#52), so
        // the systems reader reports the zone's assignment and skips it.
        assert!(
            anomalies.iter().all(|a| version == SchemaVersion::Ifc2x3
                && matches!(a, SystemAnomaly::NotASystem { group, .. } if *group == authored.zone)),
            "{schema}: {anomalies:?}"
        );
        let system = found
            .iter()
            .find(|s| s.id == authored.system)
            .expect("system");
        assert_eq!(system.members, vec![s1, s2], "{schema}");
        let (found, anomalies) = zones(&back);
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        let zone = found.iter().find(|z| z.id == authored.zone).expect("zone");
        assert_eq!(zone.members, vec![SPACE], "{schema}");
        let long_name = (version != SchemaVersion::Ifc2x3).then(|| "Thermal zone A".to_owned());
        assert_eq!(zone.long_name, long_name, "{schema}");
        let (found, anomalies) = ports(&back);
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        let port = found.iter().find(|p| p.id == p1).expect("port");
        assert_eq!(port.element, Some(s1), "{schema}");
        let (graph, anomalies) = ConnectionGraph::build(&back);
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        assert_eq!(graph.neighbours(p1), vec![p2], "{schema}");
    }
}

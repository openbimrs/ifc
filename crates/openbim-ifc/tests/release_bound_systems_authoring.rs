//! Systems records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#202).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-systems`, written to STEP, read back with `ifc-step`, and checked by
//! `ifc-validate` against the declared release's table. No record this test
//! wrote may carry an error finding. The plain writers leave the IFC2X3
//! required `IfcRoot.OwnerHistory` as `$`, which the oracle test shows the
//! validator catches.

#![cfg(all(
    feature = "validate",
    feature = "systems",
    feature = "schema",
    feature = "step"
))]

use ifc::schema::{for_version, SchemaVersion};
use ifc::systems::authoring::{
    create_distribution_element_with_owner_history, create_spatial_zone_with_owner_history,
    create_zone_with_owner_history, DistributionElementKind, ElementAttributes,
};
use ifc::systems::{
    assign_to_group_with_owner_history, connect_port_to_element_with_owner_history,
    connect_ports_with_owner_history, contain_in_spatial_structure_with_owner_history,
    create_classified_system_with_owner_history, create_group_with_owner_history,
    create_port_with_owner_history, create_system, create_system_with_owner_history,
    nest_ports_with_owner_history, reference_in_spatial_structure_with_owner_history,
    ClassifiedSystemDraft, SystemKind,
};
use ifc::{Codec, Model, StepCodec};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);
const STOREY: EntityId = EntityId(20);
const SPACE: EntityId = EntityId(21);

/// Actors, an owner history (`#5`), a storey (`#20`) and a space (`#21`).
fn base(schema: &str, version: SchemaVersion) -> Model {
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
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    format!("2YvctVUKr0kugbFTf53O0{}", DIGITS[n] as char)
}

/// Every writer, with what `version` can hold.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let g = guid;
    let mut tx = Transaction::new(model);
    let m = &*model;
    let element = |name| ElementAttributes {
        name: Some(name),
        tag: Some("T"),
        ..ElementAttributes::default()
    };
    let system =
        create_system_with_owner_history(&mut tx, m, &g(1), Some("Heating"), OWNER).unwrap();
    let group =
        create_group_with_owner_history(&mut tx, m, &g(2), Some("Lot"), None, OWNER).unwrap();
    let zone = create_zone_with_owner_history(
        &mut tx,
        m,
        &g(3),
        Some("A"),
        None,
        ifc4.then_some("Zone A"),
        OWNER,
    )
    .unwrap();
    let segment = DistributionElementKind::FlowSegment;
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
        DistributionElementKind::FlowFitting,
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
        written.push(nest_ports_with_owner_history(&mut tx, m, &g(9), s1, &[p1], OWNER).unwrap());
    } else {
        written.push(
            connect_port_to_element_with_owner_history(&mut tx, m, &g(9), p1, s1, OWNER).unwrap(),
        );
    }
    written.push(
        connect_port_to_element_with_owner_history(&mut tx, m, &g(10), p2, s2, OWNER).unwrap(),
    );
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
        written.push(
            create_spatial_zone_with_owner_history(
                &mut tx,
                m,
                &g(16),
                element("Thermal"),
                Some("Thermal zone"),
                Some("THERMAL"),
                None,
                OWNER,
            )
            .unwrap(),
        );
        let heating = ClassifiedSystemDraft::new()
            .predefined_type("HEATING")
            .long_name("Heating water");
        let kinds = [
            (SystemKind::DistributionSystem, heating),
            (SystemKind::DistributionCircuit, heating),
            (
                if version == SchemaVersion::Ifc4x3 {
                    SystemKind::Built
                } else {
                    SystemKind::Building
                },
                ClassifiedSystemDraft::new().predefined_type("FOUNDATION"),
            ),
        ];
        for (n, (kind, draft)) in kinds.into_iter().enumerate() {
            written.push(
                create_classified_system_with_owner_history(
                    &mut tx,
                    m,
                    kind,
                    &g(17 + n),
                    Some("S"),
                    draft,
                    OWNER,
                )
                .unwrap(),
            );
        }
    }
    tx.commit(model).expect("commit");
    written
}

/// Error findings `ifc-validate` reports on `ids`, against `version`.
fn errors(model: &Model, version: SchemaVersion, ids: &[EntityId]) -> Vec<String> {
    let report = ifc_validate::validate(model, for_version(version).expect("bundled"));
    report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .filter(|finding| match &finding.path {
            ifc_validate::Path::Entity(id) | ifc_validate::Path::Attribute { entity: id, .. } => {
                ids.contains(id)
            }
            ifc_validate::Path::File => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_systems_records_validate_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    }
}

/// The oracle is trusted because it fails when it should: the plain
/// writer's `$` `OwnerHistory` is an error finding in IFC2X3.
#[test]
fn the_validator_catches_an_unowned_ifc2x3_system() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let system = create_system(&mut tx, &guid(1), Some("Heating")).unwrap();
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[system]).is_empty(),
        "IFC2X3 requires IfcRoot.OwnerHistory"
    );
}

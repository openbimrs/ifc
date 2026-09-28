//! Release-bound systems writers leave IFC4 and IFC4X3 output unchanged
//! (#202): each plain writer still writes its pre-#202 record, and its
//! `*_with_owner_history` variant writes the same record with the owner
//! history in `IfcRoot.OwnerHistory` (slot 1).

use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction, Value};
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
    nest_ports_with_owner_history, reference_in_spatial_structure_with_owner_history,
    ClassifiedSystemDraft, SchemaVersion, SystemKind,
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

fn staged(tx: &Transaction, id: EntityId) -> Entity {
    tx.edits()
        .iter()
        .find_map(|edit| match edit {
            Edit::Create { id: staged, entity } if *staged == id => Some(entity.clone()),
            _ => None,
        })
        .expect("staged")
}

/// IFC4 and IFC4X3 output is unchanged: each plain writer's record is the
/// pre-#202 layout, slot for slot, and its variant writes the same record
/// with the owner history in slot 1.
#[test]
#[allow(clippy::too_many_lines)]
fn ifc4_and_ifc4x3_records_are_unchanged() {
    let t = |s: &str| Value::Text(s.into());
    let e = |s: &str| Value::Enum(s.into());
    let n = Value::Null;
    let r = Value::Ref;
    for (schema, version) in &RELEASES[1..] {
        let model = base(schema, *version);
        let table = for_version(*version).unwrap();
        let mut tx = Transaction::new(&model);
        let m = &model;
        let g = guid(1);
        let g = g.as_str();
        let attrs = ElementAttributes {
            name: Some("P"),
            description: Some("D"),
            placement: Some(STOREY),
            representation: None,
            tag: Some("T"),
        };
        let kind = DistributionElementKind::FlowSegment;
        let draft = ClassifiedSystemDraft {
            description: Some("D"),
            object_type: None,
            predefined_type: Some("HEATING"),
            long_name: Some("L"),
        };
        let x = ifc_systems::authoring::create_distribution_element; // plain
        let pairs: Vec<(EntityId, EntityId, Vec<Value>)> = vec![
            (
                ifc_systems::create_system(&mut tx, g, Some("S")).unwrap(),
                create_system_with_owner_history(&mut tx, m, g, Some("S"), OWNER).unwrap(),
                vec![t(g), n.clone(), t("S"), n.clone(), n.clone()],
            ),
            (
                ifc_systems::create_group(&mut tx, g, Some("G"), Some("D")).unwrap(),
                create_group_with_owner_history(&mut tx, m, g, Some("G"), Some("D"), OWNER)
                    .unwrap(),
                vec![t(g), n.clone(), t("G"), t("D"), n.clone()],
            ),
            (
                ifc_systems::create_port(&mut tx, g, Some("P"), Some("SINK")).unwrap(),
                create_port_with_owner_history(&mut tx, m, g, Some("P"), Some("SINK"), OWNER)
                    .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    t("P"),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    e("SINK"),
                    n.clone(),
                    n.clone(),
                ],
            ),
            (
                ifc_systems::assign_to_group(&mut tx, g, SPACE, &[STOREY]).unwrap(),
                assign_to_group_with_owner_history(&mut tx, m, g, SPACE, &[STOREY], OWNER).unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    Value::List(vec![r(STOREY)]),
                    n.clone(),
                    r(SPACE),
                ],
            ),
            (
                ifc_systems::nest_ports(&mut tx, g, STOREY, &[SPACE]).unwrap(),
                nest_ports_with_owner_history(&mut tx, m, g, STOREY, &[SPACE], OWNER).unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    r(STOREY),
                    Value::List(vec![r(SPACE)]),
                ],
            ),
            (
                ifc_systems::connect_port_to_element(&mut tx, g, SPACE, STOREY).unwrap(),
                connect_port_to_element_with_owner_history(&mut tx, m, g, SPACE, STOREY, OWNER)
                    .unwrap(),
                vec![t(g), n.clone(), n.clone(), n.clone(), r(SPACE), r(STOREY)],
            ),
            (
                ifc_systems::connect_ports(&mut tx, g, SPACE, STOREY, Some(OWNER)).unwrap(),
                connect_ports_with_owner_history(&mut tx, m, g, SPACE, STOREY, Some(OWNER), OWNER)
                    .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    r(SPACE),
                    r(STOREY),
                    r(OWNER),
                ],
            ),
            (
                ifc_systems::contain_in_spatial_structure(&mut tx, g, STOREY, &[SPACE]).unwrap(),
                contain_in_spatial_structure_with_owner_history(
                    &mut tx,
                    m,
                    g,
                    STOREY,
                    &[SPACE],
                    OWNER,
                )
                .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    Value::List(vec![r(SPACE)]),
                    r(STOREY),
                ],
            ),
            (
                ifc_systems::reference_in_spatial_structure(&mut tx, g, STOREY, &[SPACE]).unwrap(),
                reference_in_spatial_structure_with_owner_history(
                    &mut tx,
                    m,
                    g,
                    STOREY,
                    &[SPACE],
                    OWNER,
                )
                .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    Value::List(vec![r(SPACE)]),
                    r(STOREY),
                ],
            ),
            (
                x(&mut tx, kind, g, attrs).unwrap(),
                create_distribution_element_with_owner_history(&mut tx, m, kind, g, attrs, OWNER)
                    .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    t("P"),
                    t("D"),
                    n.clone(),
                    r(STOREY),
                    n.clone(),
                    t("T"),
                ],
            ),
            (
                ifc_systems::authoring::create_zone(&mut tx, g, Some("Z"), None, Some("L"))
                    .unwrap(),
                create_zone_with_owner_history(&mut tx, m, g, Some("Z"), None, Some("L"), OWNER)
                    .unwrap(),
                vec![t(g), n.clone(), t("Z"), n.clone(), n.clone(), t("L")],
            ),
            (
                ifc_systems::authoring::create_spatial_zone(
                    &mut tx,
                    g,
                    attrs,
                    Some("L"),
                    Some("USERDEFINED"),
                    Some("Kind"),
                )
                .unwrap(),
                create_spatial_zone_with_owner_history(
                    &mut tx,
                    m,
                    g,
                    attrs,
                    Some("L"),
                    Some("USERDEFINED"),
                    Some("Kind"),
                    OWNER,
                )
                .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    t("P"),
                    t("D"),
                    t("Kind"),
                    r(STOREY),
                    n.clone(),
                    t("L"),
                    e("USERDEFINED"),
                ],
            ),
            (
                ifc_systems::create_classified_system(
                    &mut tx,
                    table,
                    SystemKind::DistributionSystem,
                    g,
                    Some("HW"),
                    draft,
                )
                .unwrap(),
                create_classified_system_with_owner_history(
                    &mut tx,
                    m,
                    SystemKind::DistributionSystem,
                    g,
                    Some("HW"),
                    draft,
                    OWNER,
                )
                .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    t("HW"),
                    t("D"),
                    n.clone(),
                    t("L"),
                    e("HEATING"),
                ],
            ),
            (
                ifc_systems::create_classified_system(
                    &mut tx,
                    table,
                    SystemKind::Building,
                    g,
                    None,
                    ClassifiedSystemDraft {
                        predefined_type: Some("SHADING"),
                        long_name: Some("L"),
                        ..ClassifiedSystemDraft::default()
                    },
                )
                .unwrap(),
                create_classified_system_with_owner_history(
                    &mut tx,
                    m,
                    SystemKind::Building,
                    g,
                    None,
                    ClassifiedSystemDraft {
                        predefined_type: Some("SHADING"),
                        long_name: Some("L"),
                        ..ClassifiedSystemDraft::default()
                    },
                    OWNER,
                )
                .unwrap(),
                vec![
                    t(g),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    e("SHADING"),
                    t("L"),
                ],
            ),
        ];
        for (plain, with, mut expected) in pairs {
            let plain = staged(&tx, plain);
            assert_eq!(plain.attributes, expected, "{schema} {}", plain.type_name);
            expected[1] = r(OWNER);
            let with = staged(&tx, with);
            assert_eq!(with.type_name, plain.type_name);
            assert_eq!(with.attributes, expected, "{schema} {}", with.type_name);
        }
    }
}

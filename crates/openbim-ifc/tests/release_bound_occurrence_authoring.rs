//! The occurrences and type definitions whose required attributes the
//! drafts could not carry before #214, authored in the release that
//! requires them and validated against it.
//!
//! IFC2X3 TC1 requires `ShapeType` on `IfcRamp`, `IfcRoof` and `IfcStair`,
//! the bar measures of `IfcReinforcingBar`, `IfcTendon` and
//! `IfcReinforcingMesh`, and `IfcFurnitureType.AssemblyPlace`, and types an
//! `IfcDoor` by an `IfcDoorStyle`; IFC4 ADD2 TC1 and IFC4X3 ADD2 require
//! `IfcDoorType.OperationType`, `IfcWindowType.PartitioningType`,
//! `IfcEventType.EventTriggerType` and `IfcFurnitureType.AssemblyPlace`.
//! Each record is written with the `*_with_owner_history` writers, written
//! to STEP, read back with `ifc-step`, read back by attribute name, and
//! checked by `ifc-validate` against the declared release. No record this
//! test wrote may carry an error finding.

#![cfg(all(
    feature = "validate",
    feature = "schema",
    feature = "step",
    feature = "occurrence",
    feature = "element-type"
))]

use ifc::element_type::table::{IFCDOORTYPE, IFCEVENTTYPE, IFCFURNITURETYPE, IFCWINDOWTYPE};
use ifc::element_type::{create_type_with_owner_history, ElementType, TypeDraft};
use ifc::occurrence::table::{
    Occurrence, IFCDOOR, IFCRAMP, IFCREINFORCINGBAR, IFCREINFORCINGMESH, IFCROOF, IFCSTAIR,
    IFCTENDON,
};
use ifc::occurrence::{create_with_owner_history, MeshBars, OccurrenceDraft};
use ifc::schema::for_version;
use ifc::{Codec, Entity, Model, SchemaVersion, StepCodec, Value};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);
const DOOR_STYLE: EntityId = EntityId(12);

/// Actors and an owner history (`#5`) in `schema`; in IFC2X3 also a door
/// style (`#12`), which `ifc-element-type` does not author.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let style = if version == SchemaVersion::Ifc2x3 {
        "#12=IFCDOORSTYLE('2xS3BCk291UvhgP2dvNsgq',#5,'DS',$,$,$,$,$,\
         .SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.);\n"
    } else {
        ""
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         {style}\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// A fresh, unique GlobalId per record.
fn guid(n: usize) -> String {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&(n as u64 + 1).to_le_bytes());
    ifc_model::guid::Guid::from_uuid(bytes).to_string()
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
            // A path kind a later ifc-validate adds names no authored record.
            _ => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

/// `attribute` of the record `id` in `model`, by `version`'s own layout.
fn read(model: &Model, version: SchemaVersion, id: EntityId, attribute: &str) -> Value {
    let record = model.get(id).expect("read back");
    let table = for_version(version).expect("bundled");
    let at = table
        .attribute_names(&record.type_name)
        .iter()
        .position(|name| *name == attribute)
        .unwrap_or_else(|| panic!("{}.{attribute}", record.type_name));
    record.attributes[at].clone()
}

/// Write `model`, read it back, and check `written` has no error finding.
fn round_trip(model: &Model, schema: &str, version: SchemaVersion, written: &[EntityId]) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    let found = errors(&back, version, written);
    assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    back
}

fn enumeration(token: &str) -> Value {
    Value::Enum(token.into())
}

/// A class, its token, the draft, and the values expected back by name.
type Case<'a> = (
    Occurrence,
    Option<&'a str>,
    OccurrenceDraft<'a>,
    Vec<(&'a str, Value)>,
);

#[test]
fn ifc2x3_occurrences_round_trip_and_validate() {
    let version = SchemaVersion::Ifc2x3;
    let mut model = base("IFC2X3", version);
    let longitudinal = MeshBars::new(0.008, 5.0e-5, 0.15);
    let transverse = MeshBars::new(0.006, 2.8e-5, 0.2);
    let cases: Vec<Case<'_>> = vec![
        (
            IFCRAMP,
            None,
            OccurrenceDraft::new()
                .name("R")
                .shape_type("STRAIGHT_RUN_RAMP"),
            vec![("ShapeType", enumeration("STRAIGHT_RUN_RAMP"))],
        ),
        (
            IFCROOF,
            None,
            OccurrenceDraft::new().name("R").shape_type("FLAT_ROOF"),
            vec![("ShapeType", enumeration("FLAT_ROOF"))],
        ),
        (
            IFCSTAIR,
            None,
            OccurrenceDraft::new()
                .name("S")
                .shape_type("STRAIGHT_RUN_STAIR"),
            vec![("ShapeType", enumeration("STRAIGHT_RUN_STAIR"))],
        ),
        (
            IFCREINFORCINGBAR,
            None,
            OccurrenceDraft::new()
                .name("B")
                .nominal_diameter(0.012)
                .cross_section_area(1.13e-4)
                .bar_role("MAIN"),
            vec![
                ("NominalDiameter", Value::Real(0.012)),
                ("CrossSectionArea", Value::Real(1.13e-4)),
                ("BarRole", enumeration("MAIN")),
            ],
        ),
        (
            IFCTENDON,
            Some("STRAND"),
            OccurrenceDraft::new()
                .name("T")
                .nominal_diameter(0.015)
                .cross_section_area(1.4e-4),
            vec![
                ("PredefinedType", enumeration("STRAND")),
                ("NominalDiameter", Value::Real(0.015)),
                ("CrossSectionArea", Value::Real(1.4e-4)),
            ],
        ),
        (
            IFCREINFORCINGMESH,
            None,
            OccurrenceDraft::new()
                .name("M")
                .longitudinal_bars(longitudinal)
                .transverse_bars(transverse),
            vec![
                ("LongitudinalBarNominalDiameter", Value::Real(0.008)),
                ("TransverseBarNominalDiameter", Value::Real(0.006)),
                ("LongitudinalBarCrossSectionArea", Value::Real(5.0e-5)),
                ("TransverseBarCrossSectionArea", Value::Real(2.8e-5)),
                ("LongitudinalBarSpacing", Value::Real(0.15)),
                ("TransverseBarSpacing", Value::Real(0.2)),
            ],
        ),
    ];
    let mut tx = Transaction::new(&model);
    let mut written = Vec::new();
    for (n, (kind, token, draft, _)) in cases.iter().enumerate() {
        let id = create_with_owner_history(
            &mut tx,
            &model,
            *kind,
            &guid(n),
            *token,
            None,
            *draft,
            OWNER,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", kind.type_name));
        written.push(id);
    }
    // An IFC2X3 door takes an `IfcDoorStyle` (#214), related to it by an
    // `IfcRelDefinesByType` in IFC2X3's own layout.
    let door = create_with_owner_history(
        &mut tx,
        &model,
        IFCDOOR,
        &guid(100),
        None,
        Some(DOOR_STYLE),
        OccurrenceDraft::new().name("D-01"),
        OWNER,
    )
    .expect("an IFC2X3 door typed by its style");
    let names = for_version(version)
        .unwrap()
        .attribute_names("IFCRELDEFINESBYTYPE");
    let mut relation = vec![Value::Null; names.len()];
    relation[0] = Value::Text(guid(101).into());
    relation[1] = Value::Ref(OWNER);
    relation[names.iter().position(|n| *n == "RelatedObjects").unwrap()] =
        Value::List(vec![Value::Ref(door)]);
    relation[names.iter().position(|n| *n == "RelatingType").unwrap()] = Value::Ref(DOOR_STYLE);
    let relation = tx.create(Entity::new("IFCRELDEFINESBYTYPE", relation));
    written.extend([door, relation]);
    tx.commit(&mut model).expect("commit");

    let back = round_trip(&model, "IFC2X3", version, &written);
    for ((kind, _, _, expected), id) in cases.iter().zip(&written) {
        assert_eq!(back.get(*id).unwrap().type_name.as_ref(), kind.type_name);
        assert_eq!(read(&back, version, *id, "OwnerHistory"), Value::Ref(OWNER));
        for (attribute, value) in expected {
            assert_eq!(
                &read(&back, version, *id, attribute),
                value,
                "{}.{attribute}",
                kind.type_name
            );
        }
    }
    assert_eq!(
        read(&back, version, relation, "RelatingType"),
        Value::Ref(DOOR_STYLE)
    );
    assert_eq!(
        back.get(DOOR_STYLE).unwrap().type_name.as_ref(),
        "IFCDOORSTYLE"
    );
}

/// The four type definitions with a required type-specific attribute, in
/// every release that declares them.
#[test]
fn type_specific_attributes_round_trip_and_validate() {
    let cases: [(ElementType, &str, TypeDraft<'static>, &str, &str); 4] = [
        (
            IFCDOORTYPE,
            "DOOR",
            TypeDraft::new()
                .name("Door")
                .operation_type("DOUBLE_DOOR_SINGLE_SWING")
                .parameter_takes_precedence(true),
            "OperationType",
            "DOUBLE_DOOR_SINGLE_SWING",
        ),
        (
            IFCWINDOWTYPE,
            "WINDOW",
            TypeDraft::new()
                .name("Window")
                .partitioning_type("USERDEFINED")
                .user_defined_partitioning_type("Five lights"),
            "PartitioningType",
            "USERDEFINED",
        ),
        (
            IFCEVENTTYPE,
            "STARTEVENT",
            TypeDraft::new()
                .name("Start")
                .event_trigger_type("USERDEFINED")
                .user_defined_event_trigger_type("Permit issued"),
            "EventTriggerType",
            "USERDEFINED",
        ),
        (
            IFCFURNITURETYPE,
            "CHAIR",
            TypeDraft::new().name("Chair").assembly_place("FACTORY"),
            "AssemblyPlace",
            "FACTORY",
        ),
    ];
    let mut total = 0;
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let table = for_version(version).unwrap();
        let mut model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let mut written = Vec::new();
        for (n, (kind, token, draft, attribute, value)) in cases.iter().enumerate() {
            if table.entity(kind.type_name).is_none() {
                continue;
            }
            // IFC2X3's `IfcFurnitureType` declares no `PredefinedType`.
            let token = (table
                .attribute_names(kind.type_name)
                .contains(&"PredefinedType"))
            .then_some(*token);
            let id = create_type_with_owner_history(
                &mut tx,
                &model,
                *kind,
                &guid(n),
                token,
                *draft,
                OWNER,
            )
            .unwrap_or_else(|error| panic!("{schema}: {}: {error:?}", kind.type_name));
            written.push((id, *attribute, *value));
        }
        tx.commit(&mut model).expect("commit");
        let ids: Vec<EntityId> = written.iter().map(|(id, ..)| *id).collect();
        let back = round_trip(&model, schema, version, &ids);
        for (id, attribute, value) in &written {
            assert_eq!(
                read(&back, version, *id, attribute),
                enumeration(value),
                "{schema}"
            );
        }
        total += written.len();
    }
    // Furniture in all three releases; door, window and event from IFC4 on.
    assert_eq!(total, 9);
}

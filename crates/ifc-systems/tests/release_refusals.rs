//! What a release cannot hold is refused by the release-bound systems
//! writers with a typed error, staging nothing (#202).

use ifc_model::{Codec, EntityId, Model, Transaction};
use ifc_step::StepCodec;
use ifc_systems::authoring::{
    create_spatial_zone_with_owner_history, create_zone_with_owner_history, ElementAttributes,
};
use ifc_systems::{
    create_classified_system_with_owner_history, create_port_with_owner_history,
    create_system_with_owner_history, ClassifiedSystemDraft, SchemaVersion, SystemAuthoringError,
    SystemKind,
};

const GUID: &str = "2YvctVUKr0kugbFTf53O01";
const OWNER: EntityId = EntityId(5);
const STOREY: EntityId = EntityId(20);

/// An owner history (`#5`) and a storey (`#20`) in `schema`.
fn base(schema: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #5=IFCOWNERHISTORY($,$,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #20=IFCBUILDINGSTOREY('3YvctVUKr0kugbFTf53O00',#5,'L1',$,$,$,$,$,.ELEMENT.,$);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn refused(
    tx: &Transaction,
    result: Result<EntityId, SystemAuthoringError>,
) -> SystemAuthoringError {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

/// IFC2X3 refuses an attribute and the entities it does not declare.
#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let v = SchemaVersion::Ifc2x3;
    let result =
        create_zone_with_owner_history(&mut tx, &model, GUID, Some("Z"), None, Some("L"), OWNER);
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::AuthoringNotInSchema {
            entity: "IFCZONE",
            attribute: "LongName",
            schema: v,
        }
    );
    let result = create_spatial_zone_with_owner_history(
        &mut tx,
        &model,
        GUID,
        ElementAttributes::default(),
        None,
        None,
        None,
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::EntityNotInSchema {
            entity: "IFCSPATIALZONE",
            schema: v,
        }
    );
    for (kind, entity) in [
        (SystemKind::Building, "IFCBUILDINGSYSTEM"),
        (SystemKind::Built, "IFCBUILTSYSTEM"),
        (SystemKind::DistributionSystem, "IFCDISTRIBUTIONSYSTEM"),
        (SystemKind::DistributionCircuit, "IFCDISTRIBUTIONCIRCUIT"),
    ] {
        let result = create_classified_system_with_owner_history(
            &mut tx,
            &model,
            kind,
            GUID,
            None,
            ClassifiedSystemDraft::default(),
            OWNER,
        );
        assert_eq!(
            refused(&tx, result),
            SystemAuthoringError::EntityNotInSchema { entity, schema: v }
        );
    }
}

/// Tokens come from the release's own enumerations.
#[test]
fn tokens_the_release_lacks_are_refused() {
    let ifc4 = base("IFC4");
    let mut tx = Transaction::new(&ifc4);
    // IfcSpatialZoneTypeEnum gained INTERFERENCE in IFC4X3.
    let result = create_spatial_zone_with_owner_history(
        &mut tx,
        &ifc4,
        GUID,
        ElementAttributes::default(),
        None,
        Some("INTERFERENCE"),
        None,
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::AuthoringValueType {
            entity: "IFCSPATIALZONE",
            attribute: "PredefinedType",
            declared: "IfcSpatialZoneTypeEnum",
            schema: SchemaVersion::Ifc4,
        }
    );
    let result = create_classified_system_with_owner_history(
        &mut tx,
        &ifc4,
        SystemKind::Built,
        GUID,
        None,
        ClassifiedSystemDraft::default(),
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::EntityNotInSchema {
            entity: "IFCBUILTSYSTEM",
            schema: SchemaVersion::Ifc4,
        }
    );
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let model = base(schema);
        let mut tx = Transaction::new(&model);
        let result =
            create_port_with_owner_history(&mut tx, &model, GUID, None, Some("UPSTREAM"), OWNER);
        assert!(
            matches!(
                refused(&tx, result),
                SystemAuthoringError::AuthoringValueType {
                    attribute: "FlowDirection",
                    ..
                }
            ),
            "{schema}"
        );
    }
}

/// The owner history must exist and be an `IfcOwnerHistory`; the header
/// must bind exactly one known release.
#[test]
fn owner_history_and_binding_are_checked() {
    let model = base("IFC2X3");
    let mut tx = Transaction::new(&model);
    let result = create_system_with_owner_history(&mut tx, &model, GUID, None, STOREY);
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::WrongReferenceType {
            entity: "IFCSYSTEM",
            attribute: "OwnerHistory",
            target: STOREY,
            actual: "IFCBUILDINGSTOREY".into(),
            expected: "IFCOWNERHISTORY",
        }
    );
    let result = create_system_with_owner_history(&mut tx, &model, GUID, None, EntityId(99));
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::MissingReference {
            entity: "IFCSYSTEM",
            attribute: "OwnerHistory",
            target: EntityId(99),
        }
    );
    let mut several = base("IFC4");
    several.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    let result = create_system_with_owner_history(&mut tx, &several, GUID, None, OWNER);
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::MultipleSchemas { schemas: 2 }
    );
    let mut unknown = base("IFC4");
    unknown.header_mut().schema = vec!["IFC5".into()];
    let result = create_system_with_owner_history(&mut tx, &unknown, GUID, None, OWNER);
    assert_eq!(
        refused(&tx, result),
        SystemAuthoringError::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );
    // A model with no FILE_SCHEMA binds IFC4, where the owner history is
    // optional but still checked.
    let mut bare = base("IFC4");
    bare.header_mut().schema.clear();
    create_system_with_owner_history(&mut tx, &bare, GUID, None, OWNER).expect("IFC4 binding");
    assert_eq!(tx.len(), 1);
}

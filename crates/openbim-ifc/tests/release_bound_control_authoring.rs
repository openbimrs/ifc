//! Control records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#198, #202).
//!
//! Each release is authored through the `*_with_owner_history` writers of
//! `ifc-control`, written to STEP, read back with `ifc-step`, and checked by
//! `ifc-validate` against the declared release's table. No record this test
//! wrote may carry an error finding. Before #198 the IFC2X3 permit, action
//! request and performance history panicked in the writer.

#![cfg(all(
    feature = "validate",
    feature = "control",
    feature = "schema",
    feature = "step"
))]

use ifc::control::{
    assign_to_control_with_owner_history, create_control_with_owner_history,
    ControlAssignmentDraft, ControlDraft, ControlKind,
};
use ifc::schema::{for_version, SchemaVersion};
use ifc::{Codec, Model, StepCodec};
use ifc_model::{EntityId, Transaction};

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);

/// Actors, an owner history (`#5`) and a wall (`#10`) in `schema`.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let unset = ",$".repeat(for_version(version).unwrap().attributes("IFCWALL").len() - 3);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{unset});\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// Every control, and an assignment to each, with what `version` can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let guids = [
        "0YvctVUKr0kugbFTf53O08",
        "0YvctVUKr0kugbFTf53O09",
        "0YvctVUKr0kugbFTf53O0A",
        "0YvctVUKr0kugbFTf53O0B",
    ];
    let relations = [
        "0YvctVUKr0kugbFTf53O0C",
        "0YvctVUKr0kugbFTf53O0D",
        "0YvctVUKr0kugbFTf53O0E",
        "0YvctVUKr0kugbFTf53O0F",
    ];
    let mut tx = Transaction::new(model);
    let mut written = Vec::new();
    for ((kind, guid), relation) in ControlKind::ALL.into_iter().zip(guids).zip(relations) {
        let history = kind == ControlKind::PerformanceHistory;
        let predefined = match kind {
            ControlKind::ProjectOrder => Some("WORKORDER"),
            _ if ifc4 => Some("NOTDEFINED"),
            _ => None,
        };
        let draft = ControlDraft {
            name: Some("Control"),
            description: Some("East wing"),
            object_type: None,
            identification: (ifc4 || !history).then_some("C-1"),
            status: (!history && (ifc4 || kind == ControlKind::ProjectOrder)).then_some("OPEN"),
            long_description: (ifc4 && !history).then_some("Long"),
            life_cycle_phase: history.then_some("OPERATION"),
        };
        let control =
            create_control_with_owner_history(&mut tx, model, kind, guid, predefined, draft, OWNER)
                .expect("control");
        let assignment = ControlAssignmentDraft {
            global_id: relation,
            name: None,
            description: None,
            control,
            related_objects: &[WALL],
        };
        let assigned = assign_to_control_with_owner_history(&mut tx, model, assignment, OWNER)
            .expect("assignment");
        written.extend([control, assigned]);
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
fn authored_control_records_validate_in_their_release() {
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

/// The oracle is trusted because it fails when it should: an IFC2X3 permit
/// with `$` for its required `OwnerHistory` is an error finding.
#[test]
fn the_validator_catches_an_unowned_ifc2x3_permit() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let text = |s: &str| ifc::Value::Text(s.into());
    let permit = tx.create(ifc::Entity::new(
        "IFCPERMIT",
        vec![
            text("0YvctVUKr0kugbFTf53O08"),
            ifc::Value::Null,
            text("Control"),
            ifc::Value::Null,
            ifc::Value::Null,
            text("C-1"),
        ],
    ));
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[permit]).is_empty(),
        "IFC2X3 requires IfcRoot.OwnerHistory"
    );
}

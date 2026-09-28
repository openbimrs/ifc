//! #202: `IfcRelAssociatesApproval` is written in the declared release.
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2. Each release is authored, written as STEP, read back with
//! `ifc-step` and re-read through the view. `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs the same records through
//! `ifc-validate`.

use ifc_approval::{
    associate_approval, associate_approval_with_owner_history, ApprovalAssociationDraft,
    ApprovalError, ApprovalView,
};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, SchemaVersion};
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const APPROVAL: EntityId = EntityId(20);
const G1: &str = "0YvctVUKr0kugbFTf53O08";

/// `#5` an owner history, `#10` a wall and `#20` an approval, each in the
/// layout of `schema`; `schema = None` leaves `FILE_SCHEMA` empty.
fn model(schema: &[&str]) -> Model {
    let version = match schema {
        [token] => SchemaVersion::from_header_token(token),
        _ => None,
    }
    .unwrap_or(SchemaVersion::Ifc4);
    let wall_tail = ",$".repeat(for_version(version).unwrap().attributes("IFCWALL").len() - 3);
    let approval = if version == SchemaVersion::Ifc2x3 {
        "#21=IFCCALENDARDATE(28,9,2026);\n#20=IFCAPPROVAL($,#21,$,$,$,'Accepted','A-1');"
    } else {
        "#20=IFCAPPROVAL('A-1','Accepted',$,$,$,$,$,$,$);"
    };
    let declared = schema
        .iter()
        .map(|token| format!("'{token}'"))
        .collect::<Vec<_>>()
        .join(",");
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(({declared}));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{wall_tail});\n\
         {approval}\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn draft(related: &[EntityId]) -> ApprovalAssociationDraft<'_> {
    ApprovalAssociationDraft::new(G1, related, APPROVAL).name("Approved")
}

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, ApprovalError>,
) -> ApprovalError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

/// Author with an owner history in every release (and without one where
/// the release allows it), write STEP, read it back and re-read the record
/// by attribute name and through the view.
#[test]
fn every_release_round_trips_with_its_own_layout() {
    for (schema, version) in RELEASES {
        let table = for_version(version).unwrap();
        let mut model = model(&[schema]);
        let mut tx = Transaction::new(&model);
        let owned = associate_approval_with_owner_history(&mut tx, &model, draft(&[WALL]), OWNER)
            .expect(schema);
        let unowned = (version != SchemaVersion::Ifc2x3)
            .then(|| associate_approval(&mut tx, &model, draft(&[WALL])).expect(schema));
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let names = table.attribute_names("IFCRELASSOCIATESAPPROVAL");
        let at = |name: &str| names.iter().position(|found| *found == name).unwrap();
        let record = back.get(owned).expect("read back");
        assert_eq!(record.attributes.len(), names.len(), "{schema}");
        assert_eq!(record.attributes[at("OwnerHistory")], Value::Ref(OWNER));
        assert_eq!(
            record.attributes[at("RelatingApproval")],
            Value::Ref(APPROVAL)
        );

        let view = ApprovalView::new(&back);
        let assignment = view.approval_assignment(owned).expect(schema);
        assert_eq!(assignment.global_id().unwrap(), G1);
        assert_eq!(assignment.related_objects().unwrap(), [WALL]);
        assert_eq!(assignment.relating_approval().unwrap(), APPROVAL);
        // The `IfcApproval` projection reads the IFC4 layout only, and the
        // IFC2X3 approval record has another one; the association itself
        // has one layout in all three releases.
        if version != SchemaVersion::Ifc2x3 {
            assert!(view.objects_approved_by(APPROVAL).unwrap().contains(&WALL));
        }
        if let Some(unowned) = unowned {
            assert_eq!(
                back.get(unowned).unwrap().attributes[at("OwnerHistory")],
                Value::Null
            );
        }
    }
}

/// IFC4 and IFC4X3 output of the writer without an owner history is the
/// six-attribute record it wrote before #202.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for schema in [&["IFC4"][..], &["IFC4X3_ADD2"], &[]] {
        let model = model(schema);
        let mut tx = Transaction::new(&model);
        let id = associate_approval(&mut tx, &model, draft(&[WALL])).expect("written");
        let ifc_model::Edit::Create { id: staged, entity } = &tx.edits()[0] else {
            panic!("expected a create");
        };
        assert_eq!(*staged, id);
        let expected = Entity::new(
            "IFCRELASSOCIATESAPPROVAL",
            vec![
                Value::Text(G1.into()),
                Value::Null,
                Value::Text("Approved".into()),
                Value::Null,
                Value::List(vec![Value::Ref(WALL)]),
                Value::Ref(APPROVAL),
            ],
        );
        assert_eq!(*entity, expected, "{schema:?}");
    }
}

#[test]
fn ifc2x3_without_an_owner_history_is_refused() {
    let model = model(&["IFC2X3"]);
    assert_eq!(
        refused(&model, |tx| associate_approval(tx, &model, draft(&[WALL]))),
        ApprovalError::AuthoringRequired {
            entity: "IFCRELASSOCIATESAPPROVAL",
            attribute: "OwnerHistory",
            schema: SchemaVersion::Ifc2x3,
        }
    );
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, _) in RELEASES {
        let model = model(&[schema]);
        assert_eq!(
            refused(&model, |tx| associate_approval_with_owner_history(
                tx,
                &model,
                draft(&[WALL]),
                WALL
            )),
            ApprovalError::AuthoringReferenceType {
                target: WALL,
                expected: "IfcOwnerHistory",
                actual: "IFCWALL".into(),
            },
            "{schema}"
        );
        let missing = EntityId(999);
        assert_eq!(
            refused(&model, |tx| associate_approval_with_owner_history(
                tx,
                &model,
                draft(&[WALL]),
                missing
            )),
            ApprovalError::UnknownEntity { id: missing },
            "{schema}"
        );
    }
}

/// An owner history staged in the same transaction is accepted.
#[test]
fn a_staged_owner_history_is_accepted() {
    let model = model(&["IFC2X3"]);
    let mut tx = Transaction::new(&model);
    let staged = tx.create(model.get(OWNER).unwrap().clone());
    associate_approval_with_owner_history(&mut tx, &model, draft(&[WALL]), staged)
        .expect("a staged IfcOwnerHistory");
}

/// IFC2X3 WR21: `RelatedObjects` is `SET OF IfcRoot`, but only object and
/// property definitions; a relationship is refused, as `IfcDefinitionSelect`
/// refuses one in IFC4.
#[test]
fn related_objects_are_checked_against_the_release() {
    for (schema, _) in RELEASES {
        let mut model = model(&[schema]);
        let mut tx = Transaction::new(&model);
        let relation =
            associate_approval_with_owner_history(&mut tx, &model, draft(&[WALL]), OWNER)
                .expect(schema);
        tx.commit(&mut model).expect("commit");
        let error = refused(&model, |tx| {
            associate_approval_with_owner_history(tx, &model, draft(&[relation]), OWNER)
        });
        assert!(
            matches!(
                error,
                ApprovalError::AuthoringReferenceType {
                    expected: "IfcDefinitionSelect",
                    ..
                }
            ),
            "{schema}: {error:?}"
        );
    }
}

#[test]
fn multiple_or_unknown_schemas_are_refused() {
    let several = model(&["IFC4", "IFC2X3"]);
    assert_eq!(
        refused(&several, |tx| associate_approval_with_owner_history(
            tx,
            &several,
            draft(&[WALL]),
            OWNER
        )),
        ApprovalError::MultipleSchemas { schemas: 2 }
    );
    let unknown = model(&["IFC9"]);
    assert_eq!(
        refused(&unknown, |tx| associate_approval(
            tx,
            &unknown,
            draft(&[WALL])
        )),
        ApprovalError::UnsupportedSchema {
            schema: "IFC9".into()
        }
    );
}

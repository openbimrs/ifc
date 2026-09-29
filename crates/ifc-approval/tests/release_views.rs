//! #212: approvals are read and written by name in the declared release.
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcApproval` with seven
//! attributes (`Description`, `ApprovalDateTime` an `IfcDateTimeSelect`,
//! `ApprovalStatus`, `ApprovalLevel`, `ApprovalQualifier`, `Name`,
//! `Identifier`; the last two and the date required) and
//! `IfcApprovalRelationship` with a single `RelatedApproval` and a required
//! `Name`; IFC4 ADD2 TC1 and IFC4X3 ADD2 declare nine and a set, and add
//! `IfcResourceApprovalRelationship`. Each release is authored, written as
//! STEP, read back with `ifc-step` and re-read through the view.

use ifc_approval::{
    associate_approval_with_owner_history, create_approval, relate_approvals,
    relate_resource_approval, ApprovalAssociationDraft, ApprovalDraft, ApprovalError,
    ApprovalRelationshipDraft, ApprovalView, DateTimeInput, ResourceApprovalDraft, SchemaVersion,
};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const DATE: EntityId = EntityId(21);
const G1: &str = "0YvctVUKr0kugbFTf53O08";

/// `#5` an owner history, `#10` a wall and, in IFC2X3, `#21` a calendar
/// date, all in the layout of `schema`.
fn model(schema: &str) -> Model {
    let wall = if schema == "IFC2X3" {
        ",$,$,$,$,$"
    } else {
        ",$,$,$,$,$,$"
    };
    let date = if schema == "IFC2X3" {
        "#21=IFCCALENDARDATE(28,9,2026);\n"
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
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{wall});\n\
         {date}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn round_trip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

/// The approval draft `version` can hold.
fn approval(version: SchemaVersion, identifier: &'static str) -> ApprovalDraft<'static> {
    let draft = ApprovalDraft::new()
        .identifier(identifier)
        .name("Design review")
        .description("Stage 3");
    if version == SchemaVersion::Ifc2x3 {
        draft.time_of_approval(DATE)
    } else {
        draft
            .time_of_approval("2026-09-28T10:00:00")
            .status("Approved")
            .level("Final")
            .qualifier("None")
            .requesting_approval(EntityId(3))
            .giving_approval(EntityId(2))
    }
}

#[test]
fn every_release_round_trips_through_the_view() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        let mut model = model(schema);
        let m = model.clone();
        let mut tx = Transaction::new(&model);
        let first = create_approval(&mut tx, &m, approval(version, "A-1")).expect("approval");
        let second = create_approval(&mut tx, &m, approval(version, "A-2")).expect("approval");
        let related = [second];
        let relationship = relate_approvals(
            &mut tx,
            &m,
            ApprovalRelationshipDraft::new(first, &related).name("Supersedes"),
        )
        .expect("relationship");
        let objects = [WALL];
        let association = associate_approval_with_owner_history(
            &mut tx,
            &m,
            ApprovalAssociationDraft::new(G1, &objects, first),
            OWNER,
        )
        .expect("association");
        let resources = [EntityId(2)];
        let resource =
            relate_resource_approval(&mut tx, &m, ResourceApprovalDraft::new(&resources, first));
        let resource = if ifc2x3 {
            assert!(matches!(
                resource,
                Err(ApprovalError::EntityNotInSchema {
                    entity: "IFCRESOURCEAPPROVALRELATIONSHIP",
                    schema: SchemaVersion::Ifc2x3
                })
            ));
            None
        } else {
            Some(resource.expect("resource relationship"))
        };
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);
        let record = |id: EntityId| back.get(id).expect("record").attributes.clone();
        if ifc2x3 {
            // Description, ApprovalDateTime, ApprovalStatus, ApprovalLevel,
            // ApprovalQualifier, Name, Identifier.
            assert_eq!(
                record(first),
                vec![
                    text("Stage 3"),
                    Value::Ref(DATE),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    text("Design review"),
                    text("A-1"),
                ]
            );
            // RelatedApproval, RelatingApproval, Description, Name.
            assert_eq!(
                record(relationship),
                vec![
                    Value::Ref(second),
                    Value::Ref(first),
                    Value::Null,
                    text("Supersedes")
                ]
            );
        } else {
            assert_eq!(record(first).len(), 9, "{schema}");
            assert_eq!(
                record(relationship),
                vec![
                    text("Supersedes"),
                    Value::Null,
                    Value::Ref(first),
                    Value::List(vec![Value::Ref(second)]),
                ],
                "{schema}"
            );
        }
        let view = ApprovalView::new(&back);
        let read = view.approval(first).expect("approval");
        assert_eq!(read.release(), version);
        assert_eq!(read.identifier().unwrap(), Some("A-1"), "{schema}");
        assert_eq!(read.name().unwrap(), Some("Design review"), "{schema}");
        assert_eq!(read.description().unwrap(), Some("Stage 3"), "{schema}");
        if ifc2x3 {
            assert_eq!(
                read.time_of_approval(),
                Err(ApprovalError::StructuredValue {
                    entity: "IFCAPPROVAL",
                    id: first,
                    attribute: "TimeOfApproval",
                    target: DATE,
                })
            );
            for (attribute, result) in [
                ("Status", read.status()),
                ("Level", read.level()),
                ("Qualifier", read.qualifier()),
            ] {
                assert_eq!(
                    result,
                    Err(ApprovalError::NotInSchema {
                        entity: "IFCAPPROVAL",
                        id: first,
                        attribute,
                        schema: version,
                    })
                );
            }
            assert!(matches!(
                read.requesting_approval(),
                Err(ApprovalError::NotInSchema { .. })
            ));
            assert!(matches!(
                read.giving_approval(),
                Err(ApprovalError::NotInSchema { .. })
            ));
        } else {
            assert_eq!(
                read.time_of_approval().unwrap(),
                Some("2026-09-28T10:00:00")
            );
            assert_eq!(read.status().unwrap(), Some("Approved"));
            assert_eq!(read.level().unwrap(), Some("Final"));
            assert_eq!(read.qualifier().unwrap(), Some("None"));
            assert_eq!(read.requesting_approval().unwrap(), Some(EntityId(3)));
            assert_eq!(read.giving_approval().unwrap(), Some(EntityId(2)));
        }
        let rel = view
            .approval_relationship(relationship)
            .expect("relationship");
        assert_eq!(rel.name().unwrap(), Some("Supersedes"));
        assert_eq!(rel.relating_approval().unwrap(), first);
        assert_eq!(rel.related_approvals().unwrap(), vec![second]);
        assert_eq!(
            view.approval_assignment(association)
                .unwrap()
                .relating_approval()
                .unwrap(),
            first
        );
        assert_eq!(view.objects_approved_by(first).unwrap(), vec![WALL]);
        match resource {
            Some(resource) => {
                assert_eq!(
                    view.resources_approved_by(first).unwrap(),
                    vec![EntityId(2)]
                );
                view.resource_approval_relationship(resource)
                    .expect("resource relationship");
            }
            None => assert!(view.resources_approved_by(first).unwrap().is_empty()),
        }
    }
}

/// IFC4 and IFC4X3 records are the positional ones the writers staged
/// before binding, slot for slot.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema);
        let mut tx = Transaction::new(&model);
        let first = create_approval(&mut tx, &model, approval(SchemaVersion::Ifc4, "A-1")).unwrap();
        let second =
            create_approval(&mut tx, &model, approval(SchemaVersion::Ifc4, "A-2")).unwrap();
        let related = [second];
        let rel = relate_approvals(
            &mut tx,
            &model,
            ApprovalRelationshipDraft::new(first, &related),
        )
        .unwrap();
        let staged = |id: EntityId| {
            tx.edits()
                .iter()
                .find_map(|edit| match edit {
                    ifc_model::Edit::Create { id: e, entity } if *e == id => Some(entity.clone()),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(
            staged(first),
            Entity::new(
                "IFCAPPROVAL",
                vec![
                    text("A-1"),
                    text("Design review"),
                    text("Stage 3"),
                    text("2026-09-28T10:00:00"),
                    text("Approved"),
                    text("Final"),
                    text("None"),
                    Value::Ref(EntityId(3)),
                    Value::Ref(EntityId(2)),
                ]
            )
        );
        assert_eq!(
            staged(rel).attributes,
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(first),
                Value::List(vec![Value::Ref(second)])
            ]
        );
    }
}

fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, ApprovalError>,
) -> ApprovalError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let v = SchemaVersion::Ifc2x3;
    let model = model("IFC2X3");
    let base = || approval(v, "A-1");
    assert_eq!(
        refused(&model, |tx| create_approval(
            tx,
            &model,
            base().status("Approved")
        )),
        ApprovalError::AuthoringNotInSchema {
            entity: "IFCAPPROVAL",
            attribute: "Status",
            schema: v
        }
    );
    assert!(matches!(
        refused(&model, |tx| create_approval(
            tx,
            &model,
            base().time_of_approval("2026-09-28T10:00:00")
        )),
        ApprovalError::AuthoringValueType {
            attribute: "TimeOfApproval",
            ..
        }
    ));
    let mut undated = base();
    undated.time_of_approval = None;
    assert_eq!(
        refused(&model, |tx| create_approval(tx, &model, undated)),
        ApprovalError::AuthoringRequired {
            entity: "IFCAPPROVAL",
            attribute: "ApprovalDateTime",
            schema: v
        }
    );
    let mut unnamed = base();
    unnamed.name = None;
    assert_eq!(
        refused(&model, |tx| create_approval(tx, &model, unnamed)),
        ApprovalError::AuthoringRequired {
            entity: "IFCAPPROVAL",
            attribute: "Name",
            schema: v
        }
    );
    // A date record must be a date record: the wall is not one.
    assert!(matches!(
        refused(&model, |tx| create_approval(
            tx,
            &model,
            base().time_of_approval(WALL)
        )),
        ApprovalError::AuthoringReferenceType { target: WALL, .. }
    ));
    let mut staged = Transaction::new(&model);
    let a = create_approval(&mut staged, &model, base()).unwrap();
    let b = create_approval(&mut staged, &model, approval(v, "A-2")).unwrap();
    let c = create_approval(&mut staged, &model, approval(v, "A-3")).unwrap();
    let before = staged.edits().len();
    let two = [b, c];
    assert!(matches!(
        relate_approvals(
            &mut staged,
            &model,
            ApprovalRelationshipDraft::new(a, &two).name("Both")
        ),
        Err(ApprovalError::AuthoringValueType {
            attribute: "RelatedApprovals",
            ..
        })
    ));
    let one = [b];
    assert_eq!(
        relate_approvals(&mut staged, &model, ApprovalRelationshipDraft::new(a, &one)),
        Err(ApprovalError::AuthoringRequired {
            entity: "IFCAPPROVALRELATIONSHIP",
            attribute: "Name",
            schema: v
        })
    );
    assert_eq!(staged.edits().len(), before, "nothing staged");
}

/// IFC4 declares `IfcDateTime` text: a record there is refused.
#[test]
fn ifc4_refuses_a_date_record() {
    let model = model("IFC4");
    let mut tx = Transaction::new(&model);
    let date = tx.create(Entity::new(
        "IFCCALENDARDATE",
        vec![Value::Integer(1), Value::Integer(1), Value::Integer(2026)],
    ));
    let before = tx.edits().len();
    let error = create_approval(
        &mut tx,
        &model,
        ApprovalDraft::new()
            .name("N")
            .time_of_approval(DateTimeInput::Record(date)),
    )
    .expect_err("refused");
    assert_eq!(
        error,
        ApprovalError::AuthoringValueType {
            entity: "IFCAPPROVAL",
            attribute: "TimeOfApproval",
            declared: "IfcDateTime",
            schema: SchemaVersion::Ifc4,
        }
    );
    assert_eq!(tx.edits().len(), before);
}

/// Unverified or ambiguous headers are refused by the writers and the view,
/// never read or written as IFC4.
#[test]
fn unverified_releases_are_refused() {
    for schema in ["IFC4X1", "IFC4X2", "IFC5"] {
        let model = model(schema);
        let unsupported = ApprovalError::UnsupportedSchema {
            schema: schema.into(),
        };
        assert_eq!(
            refused(&model, |tx| create_approval(
                tx,
                &model,
                ApprovalDraft::new().name("N")
            )),
            unsupported
        );
        let mut tx = Transaction::new(&model);
        let mut staged = model.clone();
        let id = tx.create(Entity::new("IFCAPPROVAL", vec![Value::Null; 9]));
        tx.commit(&mut staged).unwrap();
        assert_eq!(
            ApprovalView::new(&staged).approval(id).err(),
            Some(unsupported)
        );
    }
    let entity = Entity::new("IFCAPPROVAL", vec![Value::Null; 9]);
    for version in [SchemaVersion::Ifc4x1, SchemaVersion::Ifc4x2] {
        assert!(matches!(
            ifc_approval::Approval::try_new(EntityId(1), &entity, version),
            Err(ApprovalError::UnsupportedSchema { .. })
        ));
    }
}

/// The binding is what makes the answers right: the IFC2X3 record read
/// through the IFC4 layout returns its date record as the name.
#[test]
fn reading_an_ifc2x3_record_as_ifc4_would_misread_it() {
    let mut model = model("IFC2X3");
    let m = model.clone();
    let mut tx = Transaction::new(&model);
    let id = create_approval(&mut tx, &m, approval(SchemaVersion::Ifc2x3, "A-1")).unwrap();
    tx.commit(&mut model).unwrap();
    let entity = model.get(id).unwrap();
    let as_ifc4 = ifc_approval::Approval::try_new(id, entity, SchemaVersion::Ifc4).unwrap();
    assert_ne!(as_ifc4.identifier().ok().flatten(), Some("A-1"));
    let bound = ApprovalView::new(&model).approval(id).unwrap();
    assert_eq!(bound.identifier().unwrap(), Some("A-1"));
}

//! Control authoring bound to the declared release (#198, #202).
//!
//! From the EXPRESS sources: IFC2X3 TC1 declares `IfcPermit`,
//! `IfcActionRequest` and `IfcPerformanceHistory` with six attributes
//! (`PermitID`, `RequestID` and `LifeCyclePhase` after `ObjectType`) and
//! `IfcProjectOrder` with eight (`ID`, `PredefinedType`, `Status`), all
//! with a required `IfcRoot.OwnerHistory`. IFC4 ADD2 TC1 and IFC4X3 ADD2
//! declare nine (eight for the performance history), `OwnerHistory`
//! optional.

use ifc_control::{
    assign_to_control, assign_to_control_with_owner_history, create_control,
    create_control_with_owner_history, ControlAssignmentDraft, ControlDraft, ControlError,
    ControlKind,
};
use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, ifc2x3, SchemaVersion};
use ifc_step::StepCodec;

const GUID: &str = "0RSPnzHdf5hAmvCJDbRDzy";
const REL_GUID: &str = "1kTvXnbbzCWw8lcMd1dR4o";
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`) with its actors and a wall (`#10`).
fn base(schema: &str, version: SchemaVersion) -> Model {
    let arity = for_version(version).unwrap().attributes("IFCWALL").len();
    let unset = ",$".repeat(arity - 3);
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
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// The draft each kind can carry in `version`: IFC2X3 has no
/// `LongDescription`, and no `Status` on a permit or an action request.
fn draft(
    kind: ControlKind,
    version: SchemaVersion,
) -> (Option<&'static str>, ControlDraft<'static>) {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let history = kind == ControlKind::PerformanceHistory;
    let predefined = match kind {
        ControlKind::ProjectOrder => Some("WORKORDER"),
        _ if ifc4 => Some("NOTDEFINED"),
        _ => None,
    };
    let draft = ControlDraft {
        name: Some("Control"),
        description: Some("Governs the east wing"),
        object_type: None,
        identification: (ifc4 || !history).then_some("C-1"),
        status: (!history && (ifc4 || kind == ControlKind::ProjectOrder)).then_some("OPEN"),
        long_description: (ifc4 && !history).then_some("Long"),
        life_cycle_phase: history.then_some("OPERATION"),
    };
    (predefined, draft)
}

fn refused(tx: &Transaction, result: Result<EntityId, ControlError>) -> ControlError {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

/// #198: every IFC2X3 control used to panic, indexing past the six
/// attributes of a permit, an action request or a performance history.
/// Now the writer that leaves `OwnerHistory` unset refuses with a typed
/// error and stages nothing.
#[test]
fn ifc2x3_controls_no_longer_panic() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    for kind in ControlKind::ALL {
        let (predefined, draft) = draft(kind, SchemaVersion::Ifc2x3);
        let mut tx = Transaction::new(&model);
        let result = create_control(&mut tx, ifc2x3(), kind, GUID, predefined, draft);
        assert_eq!(
            refused(&tx, result),
            ControlError::AuthoringRequired {
                entity: kind.type_name(),
                attribute: "OwnerHistory".into(),
                schema: "IFC2X3".into(),
            },
            "{kind:?}"
        );
    }
}

/// Each release round-trips: written, re-read with `ifc-step`, and laid out
/// by name from that release's table.
#[test]
fn controls_round_trip_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let table = for_version(version).unwrap();
        let mut tx = Transaction::new(&model);
        let mut written = Vec::new();
        for kind in ControlKind::ALL {
            let (predefined, draft) = draft(kind, version);
            let id = create_control_with_owner_history(
                &mut tx, &model, kind, GUID, predefined, draft, OWNER,
            )
            .unwrap_or_else(|e| panic!("{schema} {kind:?}: {e}"));
            written.push((kind, id));
        }
        let permit = written[0].1;
        let assignment = ControlAssignmentDraft {
            global_id: REL_GUID,
            name: None,
            description: None,
            control: permit,
            related_objects: &[WALL],
        };
        let relation = assign_to_control_with_owner_history(&mut tx, &model, assignment, OWNER)
            .expect("assignment");
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for (kind, id) in written {
            let record = back.get(id).expect("read back");
            let names = table.attribute_names(kind.type_name());
            assert_eq!(record.attributes.len(), names.len(), "{schema} {kind:?}");
            let at =
                |name: &str| &record.attributes[names.iter().position(|n| *n == name).unwrap()];
            assert_eq!(at("OwnerHistory"), &Value::Ref(OWNER), "{schema} {kind:?}");
            assert_eq!(at("Name"), &Value::Text("Control".into()));
            let identifier = match (version, kind) {
                (SchemaVersion::Ifc2x3, ControlKind::Permit) => Some("PermitID"),
                (SchemaVersion::Ifc2x3, ControlKind::ActionRequest) => Some("RequestID"),
                (SchemaVersion::Ifc2x3, ControlKind::ProjectOrder) => Some("ID"),
                (SchemaVersion::Ifc2x3, ControlKind::PerformanceHistory) => None,
                _ => Some("Identification"),
            };
            if let Some(identifier) = identifier {
                assert_eq!(at(identifier), &Value::Text("C-1".into()), "{schema}");
            }
        }
        let relation = back.get(relation).expect("relation");
        assert_eq!(relation.attributes.len(), 7);
        assert_eq!(relation.attributes[1], Value::Ref(OWNER));
        assert_eq!(relation.attributes[6], Value::Ref(permit));
    }
}

/// IFC4 and IFC4X3 output is unchanged: the pre-#198 positional record,
/// slot for slot, from the writer that leaves `OwnerHistory` unset, and the
/// same record with the reference from the new variant.
#[test]
fn ifc4_and_ifc4x3_records_are_unchanged() {
    let text = |s: &str| Value::Text(s.into());
    let e = |s: &str| Value::Enum(s.into());
    for (schema, version) in &RELEASES[1..] {
        let model = base(schema, *version);
        let table = for_version(*version).unwrap();
        for kind in ControlKind::ALL {
            let (predefined, draft) = draft(kind, *version);
            let mut expected = vec![
                text(GUID),
                Value::Null,
                text("Control"),
                text("Governs the east wing"),
                Value::Null,
                text("C-1"),
            ];
            if kind == ControlKind::PerformanceHistory {
                expected.extend([text("OPERATION"), e("NOTDEFINED")]);
            } else {
                let token = predefined.unwrap();
                expected.extend([e(token), text("OPEN"), text("Long")]);
            }
            let mut tx = Transaction::new(&model);
            let plain = create_control(&mut tx, table, kind, GUID, predefined, draft).unwrap();
            let with = create_control_with_owner_history(
                &mut tx, &model, kind, GUID, predefined, draft, OWNER,
            )
            .unwrap();
            let staged = |id| staged(&tx, id);
            assert_eq!(staged(plain).attributes, expected, "{schema} {kind:?}");
            expected[1] = Value::Ref(OWNER);
            assert_eq!(staged(with).attributes, expected, "{schema} {kind:?}");
        }
        let mut tx = Transaction::new(&model);
        let (predefined, draft) = draft(ControlKind::Permit, *version);
        let permit =
            create_control(&mut tx, table, ControlKind::Permit, GUID, predefined, draft).unwrap();
        let assignment = ControlAssignmentDraft {
            global_id: REL_GUID,
            name: Some("Governed"),
            description: None,
            control: permit,
            related_objects: &[WALL],
        };
        let plain = assign_to_control(&mut tx, &model, table, assignment).unwrap();
        let with =
            assign_to_control_with_owner_history(&mut tx, &model, assignment, OWNER).unwrap();
        let mut expected = vec![
            text(REL_GUID),
            Value::Null,
            text("Governed"),
            Value::Null,
            Value::List(vec![Value::Ref(WALL)]),
            Value::Null,
            Value::Ref(permit),
        ];
        assert_eq!(staged(&tx, plain).attributes, expected, "{schema}");
        expected[1] = Value::Ref(OWNER);
        assert_eq!(staged(&tx, with).attributes, expected, "{schema}");
    }
}

fn staged(tx: &Transaction, id: EntityId) -> Entity {
    tx.edits()
        .iter()
        .find_map(|edit| match edit {
            ifc_model::Edit::Create { id: staged, entity } if *staged == id => Some(entity.clone()),
            _ => None,
        })
        .expect("staged")
}

/// What IFC2X3 cannot hold is refused with a typed error, staging nothing.
#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let named = ControlDraft {
        name: Some("Control"),
        identification: Some("C-1"),
        ..ControlDraft::default()
    };
    let with = |tx: &mut Transaction, kind, predefined, draft| {
        create_control_with_owner_history(tx, &model, kind, GUID, predefined, draft, OWNER)
    };
    let not_in = |entity, attribute| ControlError::AuthoringNotInSchema {
        entity,
        attribute,
        schema: "IFC2X3".into(),
    };
    let required = |entity, attribute: &str| ControlError::AuthoringRequired {
        entity,
        attribute: attribute.into(),
        schema: "IFC2X3".into(),
    };
    let result = with(&mut tx, ControlKind::Permit, Some("BUILDING"), named);
    assert_eq!(refused(&tx, result), not_in("IFCPERMIT", "PredefinedType"));
    let status = ControlDraft {
        status: Some("OPEN"),
        ..named
    };
    let result = with(&mut tx, ControlKind::ActionRequest, None, status);
    assert_eq!(refused(&tx, result), not_in("IFCACTIONREQUEST", "Status"));
    let long = ControlDraft {
        long_description: Some("Long"),
        ..named
    };
    let result = with(&mut tx, ControlKind::ProjectOrder, Some("WORKORDER"), long);
    assert_eq!(
        refused(&tx, result),
        not_in("IFCPROJECTORDER", "LongDescription")
    );
    let history = ControlDraft {
        life_cycle_phase: Some("OPERATION"),
        ..named
    };
    let result = with(&mut tx, ControlKind::PerformanceHistory, None, history);
    assert_eq!(
        refused(&tx, result),
        not_in("IFCPERFORMANCEHISTORY", "Identification")
    );
    let anonymous = ControlDraft {
        identification: None,
        ..named
    };
    let result = with(&mut tx, ControlKind::Permit, None, anonymous);
    assert_eq!(refused(&tx, result), required("IFCPERMIT", "PermitID"));
    let result = with(&mut tx, ControlKind::ProjectOrder, None, named);
    assert_eq!(
        refused(&tx, result),
        required("IFCPROJECTORDER", "PredefinedType")
    );
}

/// The owner history must exist and be an `IfcOwnerHistory`; a header must
/// bind exactly one known release.
#[test]
fn owner_history_and_binding_are_checked() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let (predefined, draft) = draft(ControlKind::Permit, SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let write = |tx: &mut Transaction, model: &Model, owner| {
        create_control_with_owner_history(
            tx,
            model,
            ControlKind::Permit,
            GUID,
            predefined,
            draft,
            owner,
        )
    };
    let result = write(&mut tx, &model, WALL);
    assert!(
        matches!(
            refused(&tx, result),
            ControlError::AuthoringInvalid {
                attribute: "OwnerHistory",
                ..
            }
        ),
        "a wall is not an owner history"
    );
    let result = write(&mut tx, &model, EntityId(99));
    assert_eq!(
        refused(&tx, result),
        ControlError::UnknownEntity { id: EntityId(99) }
    );

    let mut several = base("IFC2X3", SchemaVersion::Ifc2x3);
    several.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    let result = write(&mut tx, &several, OWNER);
    assert_eq!(
        refused(&tx, result),
        ControlError::MultipleSchemas { schemas: 2 }
    );
    let mut unknown = base("IFC2X3", SchemaVersion::Ifc2x3);
    unknown.header_mut().schema = vec!["IFC5".into()];
    let result = write(&mut tx, &unknown, OWNER);
    assert_eq!(
        refused(&tx, result),
        ControlError::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );

    let assignment = ControlAssignmentDraft {
        global_id: REL_GUID,
        name: None,
        description: None,
        control: WALL,
        related_objects: &[WALL],
    };
    let result = assign_to_control(&mut tx, &model, ifc2x3(), assignment);
    assert!(result.is_err(), "a wall is no control");
    let permit = write(&mut tx, &model, OWNER).expect("permit");
    let staged = tx.len();
    let assignment = ControlAssignmentDraft {
        control: permit,
        ..assignment
    };
    let result = assign_to_control(&mut tx, &model, ifc2x3(), assignment);
    assert_eq!(
        result.expect_err("IFC2X3 needs an owner history"),
        ControlError::AuthoringRequired {
            entity: "IFCRELASSIGNSTOCONTROL",
            attribute: "OwnerHistory".into(),
            schema: "IFC2X3".into(),
        }
    );
    let result = assign_to_control_with_owner_history(&mut tx, &model, assignment, permit);
    assert!(matches!(
        result,
        Err(ControlError::AuthoringInvalid {
            attribute: "OwnerHistory",
            ..
        })
    ));
    assert_eq!(tx.len(), staged, "refusals staged nothing");
}

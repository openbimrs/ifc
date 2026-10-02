//! Read-after-write for the four controls (#100).
//!
//! Every control is written in its release, serialised to STEP text,
//! parsed back with `ifc-step` and read through [`read_control`], which
//! binds the release from `FILE_SCHEMA` and finds each attribute by name.
//! From the EXPRESS sources: IFC2X3 TC1 declares `PermitID`, `RequestID`,
//! `ID`+`PredefinedType`+`Status` and `LifeCyclePhase` after `ObjectType`;
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 declare `Identification` on `IfcControl`
//! and `PredefinedType`, `Status`, `LongDescription` (the performance
//! history: `LifeCyclePhase`, `PredefinedType`) on each subtype.

use ifc_control::{
    assign_to_control_with_owner_history, create_control_with_owner_history, read_control,
    read_controls, ControlAssignmentDraft, ControlDraft, ControlError, ControlKind,
};
use ifc_model::{Codec, EntityId, Model, Transaction};
use ifc_schema::{for_version, SchemaVersion};
use ifc_step::StepCodec;

const GUIDS: [&str; 4] = [
    "0RSPnzHdf5hAmvCJDbRDzy",
    "2Fz8Kq0bL1tO9uJ7wXyZaB",
    "3aB4cD5eF6gH7iJ8kL9mN0",
    "1qW2eR3tY4uI5oP6aS7dF8",
];
const REL_GUID: &str = "1kTvXnbbzCWw8lcMd1dR4o";
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

fn parse(text: &str) -> Model {
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn file(schema: &str, data: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         {data}ENDSEC;\nEND-ISO-10303-21;\n"
    )
}

/// An owner history (`#5`) with its actors and a wall (`#10`).
fn base(schema: &str, version: SchemaVersion) -> Model {
    let arity = for_version(version).unwrap().attributes("IFCWALL").len();
    let unset = ",$".repeat(arity - 3);
    parse(&file(
        schema,
        &format!("#10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{unset});\n"),
    ))
}

/// What each kind carries in `version`, as the authoring draft and as the
/// values a reader must return.
#[derive(Debug, Clone, Copy)]
struct Expected {
    predefined: Option<&'static str>,
    object_type: Option<&'static str>,
    identification: Option<&'static str>,
    status: Option<&'static str>,
    long_description: Option<&'static str>,
    life_cycle_phase: Option<&'static str>,
}

fn expected(kind: ControlKind, version: SchemaVersion) -> Expected {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let history = kind == ControlKind::PerformanceHistory;
    let predefined = match kind {
        ControlKind::Permit if ifc4 => Some("BUILDING"),
        ControlKind::ProjectOrder => Some("WORKORDER"),
        ControlKind::ActionRequest if ifc4 => Some("EMAIL"),
        ControlKind::PerformanceHistory if ifc4 => Some("USERDEFINED"),
        _ => None,
    };
    Expected {
        predefined,
        object_type: Some("Chiller COP log").filter(|_| predefined == Some("USERDEFINED")),
        identification: (ifc4 || !history).then_some("C-1"),
        status: (!history && (ifc4 || kind == ControlKind::ProjectOrder)).then_some("OPEN"),
        long_description: (ifc4 && !history).then_some("Covers the east wing only."),
        life_cycle_phase: history.then_some("OPERATION"),
    }
}

fn draft(expected: Expected) -> ControlDraft<'static> {
    let mut draft = ControlDraft::new()
        .name("Control")
        .description("Governs the east wing");
    draft.object_type = expected.object_type;
    draft.identification = expected.identification;
    draft.status = expected.status;
    draft.long_description = expected.long_description;
    draft.life_cycle_phase = expected.life_cycle_phase;
    draft
}

fn round_trip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

/// Every attribute the crate authors reads back, for all four controls in
/// every release that declares them, through STEP text.
#[test]
fn every_authored_attribute_reads_back_through_step() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let mut written = Vec::new();
        for (kind, guid) in ControlKind::ALL.into_iter().zip(GUIDS) {
            let want = expected(kind, version);
            let id = create_control_with_owner_history(
                &mut tx,
                &model,
                kind,
                guid,
                want.predefined,
                draft(want),
                OWNER,
            )
            .unwrap_or_else(|e| panic!("{schema} {kind:?}: {e}"));
            written.push((kind, guid, id, want));
        }
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);

        for (kind, guid, id, want) in written {
            let at = format!("{schema} {kind:?}");
            let control = read_control(&back, id).unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(control.id(), id, "{at}");
            assert_eq!(control.kind(), kind, "{at}");
            assert_eq!(control.release(), version, "{at}");
            assert_eq!(control.global_id(), guid, "{at}");
            assert_eq!(control.owner_history(), Some(OWNER), "{at}");
            assert_eq!(control.name(), Some("Control"), "{at}");
            assert_eq!(control.description(), Some("Governs the east wing"), "{at}");
            assert_eq!(control.object_type(), want.object_type, "{at}");
            assert_eq!(control.identification(), want.identification, "{at}");
            assert_eq!(control.predefined_type(), want.predefined, "{at}");
            assert_eq!(control.status(), want.status, "{at}");
            assert_eq!(control.long_description(), want.long_description, "{at}");
            assert_eq!(control.life_cycle_phase(), want.life_cycle_phase, "{at}");

            let listed = read_controls(&back, kind).expect("listed");
            assert_eq!(
                listed.iter().map(|c| c.id()).collect::<Vec<_>>(),
                [id],
                "{at}"
            );
        }
    }
}

/// `declares` separates an attribute the release lacks from one left
/// unset, by the names the accessors use.
#[test]
fn declares_follows_the_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let mut ids = Vec::new();
        for (kind, guid) in ControlKind::ALL.into_iter().zip(GUIDS) {
            let want = expected(kind, version);
            ids.push(
                create_control_with_owner_history(
                    &mut tx,
                    &model,
                    kind,
                    guid,
                    want.predefined,
                    draft(want),
                    OWNER,
                )
                .unwrap(),
            );
        }
        tx.commit(&mut model).unwrap();
        let ifc4 = version != SchemaVersion::Ifc2x3;
        for (kind, id) in ControlKind::ALL.into_iter().zip(ids) {
            let control = read_control(&model, id).unwrap();
            let history = kind == ControlKind::PerformanceHistory;
            let at = format!("{schema} {kind:?}");
            assert_eq!(control.declares("Identification"), ifc4 || !history, "{at}");
            assert_eq!(
                control.declares("PredefinedType"),
                ifc4 || kind == ControlKind::ProjectOrder,
                "{at}"
            );
            assert_eq!(
                control.declares("Status"),
                !history && (ifc4 || kind == ControlKind::ProjectOrder),
                "{at}"
            );
            assert_eq!(
                control.declares("LongDescription"),
                ifc4 && !history,
                "{at}"
            );
            assert_eq!(control.declares("LifeCyclePhase"), history, "{at}");
        }
    }
}

/// The assignments the crate writes read back from the control they name.
#[test]
fn assignments_read_back_through_step() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let want = expected(ControlKind::Permit, version);
        let permit = create_control_with_owner_history(
            &mut tx,
            &model,
            ControlKind::Permit,
            GUIDS[0],
            want.predefined,
            draft(want),
            OWNER,
        )
        .unwrap();
        let assignment = ControlAssignmentDraft::new(REL_GUID, permit, &[WALL])
            .name("Governed")
            .description("Demolition");
        let relation =
            assign_to_control_with_owner_history(&mut tx, &model, assignment, OWNER).unwrap();
        tx.commit(&mut model).unwrap();
        let back = round_trip(&model);

        let assignments = read_control(&back, permit).unwrap().assignments().unwrap();
        assert_eq!(assignments.len(), 1, "{schema}");
        let read = assignments[0];
        assert_eq!(read.id(), relation);
        assert_eq!(read.global_id(), REL_GUID);
        assert_eq!(read.owner_history(), Some(OWNER));
        assert_eq!(read.name(), Some("Governed"));
        assert_eq!(read.description(), Some("Demolition"));
        assert_eq!(read.related_objects(), [WALL]);
    }
}

/// Identity and release refusals are typed.
#[test]
fn identity_and_release_refusals() {
    let model = base("IFC4", SchemaVersion::Ifc4);
    assert_eq!(
        read_control(&model, EntityId(99)).unwrap_err(),
        ControlError::UnknownEntity { id: EntityId(99) }
    );
    assert_eq!(
        read_control(&model, WALL).unwrap_err(),
        ControlError::ForeignControl {
            id: WALL,
            actual: "IFCWALL".into()
        }
    );
    for token in ["IFC4X1", "IFC4X2", "IFC5"] {
        let mut model = base("IFC4", SchemaVersion::Ifc4);
        model.header_mut().schema = vec![token.into()];
        assert_eq!(
            read_controls(&model, ControlKind::Permit).unwrap_err(),
            ControlError::UnsupportedSchema {
                schema: token.into()
            },
            "{token}"
        );
    }
    let mut model = base("IFC4", SchemaVersion::Ifc4);
    model.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    assert_eq!(
        read_control(&model, WALL).unwrap_err(),
        ControlError::MultipleSchemas { schemas: 2 }
    );
}

/// A record that does not fit its release is refused, never read as
/// absent or through another release's positions.
#[test]
fn records_that_do_not_fit_their_release_are_refused() {
    let id = EntityId(20);
    let read = |schema: &str, record: &str| {
        let model = parse(&file(schema, &format!("#20={record};\n")));
        read_control(&model, id).map(|_| ())
    };

    // An IFC4 permit read through IFC2X3's table has too many slots.
    assert_eq!(
        read(
            "IFC2X3",
            "IFCPERMIT('0RSPnzHdf5hAmvCJDbRDzy',#5,'P',$,$,'C-1',.BUILDING.,$,$)"
        ),
        Err(ControlError::ExtraAttributes {
            entity: "IFCPERMIT",
            id,
            declared: 6,
            found: 9,
            schema: "IFC2X3".into(),
        })
    );
    // IFC2X3 requires PermitID.
    assert_eq!(
        read("IFC2X3", "IFCPERMIT('0RSPnzHdf5hAmvCJDbRDzy',#5,'P',$,$,$)"),
        Err(ControlError::MissingAttribute {
            entity: "IFCPERMIT",
            id,
            attribute: "PermitID",
        })
    );
    // Every release requires LifeCyclePhase.
    assert_eq!(
        read(
            "IFC4X3_ADD2",
            "IFCPERFORMANCEHISTORY('0RSPnzHdf5hAmvCJDbRDzy',$,'H',$,$,$,$,.NOTDEFINED.)"
        ),
        Err(ControlError::MissingAttribute {
            entity: "IFCPERFORMANCEHISTORY",
            id,
            attribute: "LifeCyclePhase",
        })
    );
    // A token outside the release's enumeration.
    assert!(matches!(
        read(
            "IFC4",
            "IFCACTIONREQUEST('0RSPnzHdf5hAmvCJDbRDzy',$,'R',$,$,$,.WORKORDER.,$,$)"
        ),
        Err(ControlError::InvalidAttribute {
            attribute: "PredefinedType",
            ..
        })
    ));
    // Text where the predefined type belongs.
    assert!(matches!(
        read(
            "IFC4",
            "IFCPROJECTORDER('0RSPnzHdf5hAmvCJDbRDzy',$,'O',$,$,$,'WORKORDER',$,$)"
        ),
        Err(ControlError::InvalidAttribute {
            attribute: "PredefinedType",
            ..
        })
    ));
    // A short record reads its missing trailing optionals as unset.
    let model = parse(&file(
        "IFC4",
        "#20=IFCPERMIT('0RSPnzHdf5hAmvCJDbRDzy',$,'P',$,$,'C-1');\n",
    ));
    let permit = read_control(&model, id).expect("short but well-formed");
    assert_eq!(permit.identification(), Some("C-1"));
    assert_eq!(permit.status(), None);
    // An in-memory model with no FILE_SCHEMA reads as IFC4.
    let mut model = model;
    model.header_mut().schema.clear();
    assert_eq!(
        read_control(&model, id).unwrap().release(),
        SchemaVersion::Ifc4
    );
}

//! #202: occurrences are written in the model's declared release.
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2, and the three releases declare different class lists, layouts and
//! `PredefinedType` enumerations. The catalogue is IFC4X3's, so these walk
//! every row in every release, not one class. `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs a sample through
//! `ifc-validate`.

use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_occurrence::table::{Occurrence, ALL, IFCBOREHOLE, IFCDOOR, IFCSLAB, IFCWALL};
use ifc_occurrence::{create, create_with_owner_history, OccurrenceDraft, OccurrenceError};
use ifc_schema::{for_version, Schema, SchemaVersion, TypeKind};
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
const OWNER: EntityId = EntityId(5);
const WALL_TYPE: EntityId = EntityId(11);
const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

/// Actors and an owner history (`#5`), and a wall type (`#11`), declaring
/// `schema`; an empty `schema` leaves `FILE_SCHEMA` empty.
fn model(schema: &[&str]) -> Model {
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
         #11=IFCWALLTYPE('2xS3BCk291UvhgP2dvNsgp',#5,'WT',$,$,$,$,$,$,.SOLIDWALL.);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn table(version: SchemaVersion) -> &'static Schema {
    for_version(version).expect("bundled")
}

/// `entity`'s `PredefinedType` tokens in `schema`, if it declares one.
fn members(schema: &Schema, entity: &str) -> Option<Vec<String>> {
    let declared = schema
        .attributes(entity)
        .into_iter()
        .find(|attribute| attribute.name == "PredefinedType")?;
    match &schema.type_def(&declared.type_name)?.kind {
        TypeKind::Enumeration(members) => Some(members.clone()),
        _ => None,
    }
}

fn declares(schema: &Schema, entity: &str) -> bool {
    schema.entity(entity).is_some_and(|found| !found.abstract_)
}

fn slot(schema: &Schema, entity: &str, attribute: &str) -> usize {
    schema
        .attribute_names(entity)
        .iter()
        .position(|name| *name == attribute)
        .unwrap_or_else(|| panic!("{entity}.{attribute}"))
}

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, OccurrenceError>,
) -> OccurrenceError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

/// The first token of `kind` this release declares, other than
/// `USERDEFINED`, so the draft needs no `ObjectType`.
fn token(schema: &Schema, kind: Occurrence) -> Option<String> {
    members(schema, kind.type_name)?
        .into_iter()
        .find(|member| member != "USERDEFINED")
}

/// Every catalogue row, in every release, with an owner history: a row
/// the release declares is written with that release's arity, owner
/// history and token; a row it lacks is `EntityNotInSchema`; and the only
/// other refusal is an IFC2X3 attribute the draft cannot carry.
#[test]
fn the_whole_catalogue_is_written_in_every_release() {
    for (schema, version) in RELEASES {
        let table = table(version);
        let model = model(&[schema]);
        let (mut written, mut absent, mut required) = (0, 0, Vec::new());
        for kind in ALL {
            let entity = kind.type_name;
            let token = token(table, *kind);
            let mut tx = Transaction::new(&model);
            let result = create_with_owner_history(
                &mut tx,
                &model,
                *kind,
                GUID,
                token.as_deref(),
                None,
                OccurrenceDraft {
                    name: Some("Sweep"),
                    tag: Some("T-1"),
                    ..OccurrenceDraft::default()
                },
                OWNER,
            );
            match result {
                Ok(id) => {
                    assert!(declares(table, entity), "{schema}: {entity} written");
                    let ifc_model::Edit::Create {
                        id: staged,
                        entity: record,
                    } = &tx.edits()[0]
                    else {
                        panic!("expected a create");
                    };
                    assert_eq!(*staged, id);
                    let names = table.attribute_names(entity);
                    assert_eq!(record.attributes.len(), names.len(), "{schema}: {entity}");
                    let at = |name| slot(table, entity, name);
                    assert_eq!(record.attributes[at("OwnerHistory")], Value::Ref(OWNER));
                    assert_eq!(record.attributes[at("Tag")], Value::Text("T-1".into()));
                    if let Some(token) = &token {
                        assert_eq!(
                            record.attributes[at("PredefinedType")],
                            Value::Enum(token.as_str().into()),
                            "{schema}: {entity}"
                        );
                    }
                    written += 1;
                }
                Err(OccurrenceError::EntityNotInSchema {
                    entity: e,
                    schema: s,
                }) => {
                    assert_eq!((e, s), (entity, version));
                    assert!(!declares(table, entity), "{schema}: {entity} refused");
                    absent += 1;
                }
                Err(OccurrenceError::AuthoringRequired {
                    entity: e,
                    attribute,
                    schema: s,
                }) => {
                    assert_eq!((e, s), (entity, SchemaVersion::Ifc2x3), "{attribute}");
                    assert_ne!(attribute, "OwnerHistory");
                    let declared = table.attributes(entity);
                    assert!(declared
                        .iter()
                        .any(|found| found.name == attribute && !found.optional));
                    required.push(format!("{entity}.{attribute}"));
                }
                Err(other) => panic!("{schema}: {entity}: {other:?}"),
            }
            // One record per call, and none at all on a refusal.
            assert!(tx.edits().len() <= 1, "{schema}: {entity}");
        }
        assert_eq!(written + absent + required.len(), ALL.len());
        // Minimum counts, so a sweep that silently matched nothing fails.
        match version {
            SchemaVersion::Ifc4x3 => assert_eq!(written, ALL.len(), "IFC4X3 is the catalogue"),
            SchemaVersion::Ifc4 => {
                assert!(written >= 100 && absent >= 25, "{written}/{absent}");
                assert!(required.is_empty(), "{required:?}");
            }
            SchemaVersion::Ifc2x3 => {
                assert!(written >= 25 && absent >= 100, "{written}/{absent}");
                // IFC2X3 attributes IFC4 made optional or renamed; the
                // draft has no field for them, so they are refused, not
                // guessed (IFC2X3 `ShapeType` is not IFC4 `PredefinedType`).
                assert_eq!(
                    required,
                    [
                        "IFCRAMP.ShapeType",
                        "IFCREINFORCINGBAR.NominalDiameter",
                        "IFCREINFORCINGMESH.LongitudinalBarNominalDiameter",
                        "IFCROOF.ShapeType",
                        "IFCSTAIR.ShapeType",
                        "IFCTENDON.NominalDiameter",
                    ]
                );
            }
            other => unreachable!("{other:?} is not swept"),
        }
    }
}

/// The IFC4X3 catalogue's tokens are the IFC4X3 table's, so checking the
/// token against the bound table changes nothing for an IFC4X3 model.
#[test]
fn catalogue_tokens_are_the_ifc4x3_tables() {
    let table = table(SchemaVersion::Ifc4x3);
    for kind in ALL {
        let expected: Vec<String> = kind.members.iter().map(|m| (*m).to_owned()).collect();
        let found = members(table, kind.type_name).unwrap_or_default();
        assert_eq!(found, expected, "{}", kind.type_name);
        assert_eq!(table.attributes(kind.type_name).len(), kind.arity);
    }
}

/// IFC4X3 output of `create` is the record the catalogue row laid out
/// before #202, for every row; and IFC4 output too wherever IFC4 declares
/// the class with the catalogue row's layout.
#[test]
fn ifc4x3_and_ifc4_output_is_unchanged() {
    let mut compared = [0, 0];
    for (index, schema) in [&["IFC4X3_ADD2"][..], &["IFC4"]].into_iter().enumerate() {
        let version = SchemaVersion::from_header_token(schema[0]).unwrap();
        let table = table(version);
        let model = model(schema);
        for kind in ALL {
            let same_layout = declares(table, kind.type_name)
                && table.attribute_names(kind.type_name)
                    == table4x3().attribute_names(kind.type_name);
            if !same_layout {
                continue;
            }
            let token = token(table, *kind).filter(|token| kind.members.contains(&token.as_str()));
            let draft = OccurrenceDraft {
                name: Some("N"),
                description: Some("D"),
                object_type: Some("O"),
                tag: Some("T"),
                ..OccurrenceDraft::default()
            };
            let mut tx = Transaction::new(&model);
            create(&mut tx, &model, *kind, GUID, token.as_deref(), None, draft).expect("written");
            let ifc_model::Edit::Create { entity, .. } = &tx.edits()[0] else {
                panic!("expected a create");
            };
            // The pre-#202 layout: the catalogue row's arity and slots.
            let mut before = vec![Value::Null; kind.arity];
            before[0] = Value::Text(GUID.into());
            before[2] = Value::Text("N".into());
            before[3] = Value::Text("D".into());
            before[4] = Value::Text("O".into());
            before[7] = Value::Text("T".into());
            if let (Some(slot), Some(token)) = (kind.predefined_slot, &token) {
                before[slot] = Value::Enum(token.as_str().into());
            }
            assert_eq!(entity.attributes, before, "{schema:?}: {}", kind.type_name);
            compared[index] += 1;
        }
    }
    assert_eq!(compared[0], ALL.len());
    assert!(compared[1] >= 60, "{compared:?}");
}

fn table4x3() -> &'static Schema {
    table(SchemaVersion::Ifc4x3)
}

/// A sample per release written, read back with `ifc-step`, and read by
/// attribute name from that release's table.
#[test]
fn a_sample_round_trips_in_every_release() {
    for (schema, version) in RELEASES {
        let table = table(version);
        let mut model = model(&[schema]);
        let mut tx = Transaction::new(&model);
        let wall = create_with_owner_history(
            &mut tx,
            &model,
            IFCWALL,
            GUID,
            None,
            Some(WALL_TYPE),
            OccurrenceDraft {
                name: Some("W-01"),
                ..OccurrenceDraft::default()
            },
            OWNER,
        )
        .expect(schema);
        let slab = create_with_owner_history(
            &mut tx,
            &model,
            IFCSLAB,
            "2hqA$FMcT8$hVvcqsRDBzZ",
            Some("FLOOR"),
            None,
            OccurrenceDraft::default(),
            OWNER,
        )
        .expect(schema);
        tx.commit(&mut model).expect("commit");
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for (id, entity) in [(wall, "IFCWALL"), (slab, "IFCSLAB")] {
            let record = back.get(id).expect("read back");
            assert_eq!(record.type_name.as_ref(), entity);
            assert_eq!(record.attributes.len(), table.attributes(entity).len());
            assert_eq!(
                record.attributes[slot(table, entity, "OwnerHistory")],
                Value::Ref(OWNER)
            );
        }
        assert_eq!(
            back.get(slab).unwrap().attributes[slot(table, "IFCSLAB", "PredefinedType")],
            Value::Enum("FLOOR".into())
        );
    }
}

#[test]
fn ifc2x3_without_an_owner_history_is_refused() {
    let model = model(&["IFC2X3"]);
    let error = refused(&model, |tx| {
        create(
            tx,
            &model,
            IFCWALL,
            GUID,
            None,
            None,
            OccurrenceDraft::default(),
        )
    });
    assert_eq!(
        error,
        OccurrenceError::AuthoringRequired {
            entity: "IFCWALL",
            attribute: "OwnerHistory",
            schema: SchemaVersion::Ifc2x3,
        }
    );
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, _) in RELEASES {
        let model = model(&[schema]);
        let author = |tx: &mut Transaction, owner| {
            create_with_owner_history(
                tx,
                &model,
                IFCWALL,
                GUID,
                None,
                None,
                OccurrenceDraft::default(),
                owner,
            )
        };
        assert_eq!(
            refused(&model, |tx| author(tx, WALL_TYPE)),
            OccurrenceError::NotAnOwnerHistory {
                id: WALL_TYPE,
                found: "IFCWALLTYPE".into()
            },
            "{schema}"
        );
        assert_eq!(
            refused(&model, |tx| author(tx, EntityId(999))),
            OccurrenceError::UnresolvedOwnerHistory { id: EntityId(999) },
            "{schema}"
        );
    }
}

/// A class or token the bound release does not declare is refused, even
/// with an owner history; a header without `FILE_SCHEMA` binds IFC4.
#[test]
fn what_the_release_lacks_is_refused() {
    for schema in [&["IFC2X3"][..], &["IFC4"], &[]] {
        let model = model(schema);
        let version = SchemaVersion::from_header_token(schema.first().unwrap_or(&"IFC4")).unwrap();
        assert_eq!(
            refused(&model, |tx| create_with_owner_history(
                tx,
                &model,
                IFCBOREHOLE,
                GUID,
                None,
                None,
                OccurrenceDraft::default(),
                OWNER,
            )),
            OccurrenceError::EntityNotInSchema {
                entity: "IFCBOREHOLE",
                schema: version,
            },
            "{schema:?}"
        );
    }
    // IFC2X3 `IfcDoor` has no `PredefinedType`; IFC4 has no `SLIDING_DOOR`
    // or similar IFC4X3-only tokens, which the table decides, not a list.
    let ifc2x3 = model(&["IFC2X3"]);
    assert_eq!(
        refused(&ifc2x3, |tx| create_with_owner_history(
            tx,
            &ifc2x3,
            IFCDOOR,
            GUID,
            Some("DOOR"),
            None,
            OccurrenceDraft::default(),
            OWNER,
        )),
        OccurrenceError::NoPredefinedType { entity: "IFCDOOR" }
    );
    let ifc4 = model(&["IFC4"]);
    let ifc4_table = table(SchemaVersion::Ifc4);
    let (kind, token) = ALL
        .iter()
        .filter(|kind| declares(ifc4_table, kind.type_name))
        .find_map(|kind| {
            let ifc4 = members(ifc4_table, kind.type_name)?;
            let only = kind
                .members
                .iter()
                .find(|m| !ifc4.iter().any(|t| t == *m))?;
            Some((*kind, *only))
        })
        .expect("an IFC4X3-only token on a class IFC4 declares");
    assert_eq!(
        refused(&ifc4, |tx| create(
            tx,
            &ifc4,
            kind,
            GUID,
            Some(token),
            None,
            OccurrenceDraft {
                object_type: Some("x"),
                ..OccurrenceDraft::default()
            },
        )),
        OccurrenceError::UnknownPredefinedType {
            entity: kind.type_name,
            token: token.into(),
        }
    );
}

#[test]
fn multiple_or_unknown_schemas_are_refused() {
    let author = |model: &Model| {
        refused(model, |tx| {
            create(
                tx,
                model,
                IFCWALL,
                GUID,
                None,
                None,
                OccurrenceDraft::default(),
            )
        })
    };
    assert_eq!(
        author(&model(&["IFC4", "IFC2X3"])),
        OccurrenceError::MultipleSchemas { schemas: 2 }
    );
    assert_eq!(
        author(&model(&["IFC9"])),
        OccurrenceError::UnsupportedSchema {
            schema: "IFC9".into()
        }
    );
}

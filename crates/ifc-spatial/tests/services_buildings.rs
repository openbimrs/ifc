//! `IfcRelServicesBuildings` authoring checks its targets (#286).
//!
//! From the EXPRESS sources: `RelatingSystem : IfcSystem` and
//! `RelatedBuildings : SET [1:?] OF IfcSpatialStructureElement` (IFC2X3 TC1)
//! or `OF IfcSpatialElement` (IFC4 ADD2 TC1, IFC4X3 ADD2); the inverse
//! `IfcSystem.ServicesBuildings` is `SET [0:1]`. The same rules as
//! `ifc_systems::serve_buildings` (#277), in each verified release.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::SchemaVersion;
use ifc_spatial::authoring::{serve_buildings, serve_buildings_with_owner_history};
use ifc_spatial::SpatialAuthoringError;
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);
const BUILDING: EntityId = EntityId(20);
const STOREY: EntityId = EntityId(21);
const WALL: EntityId = EntityId(23);
const SYSTEM: EntityId = EntityId(30);
const OTHER_SYSTEM: EntityId = EntityId(31);

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`), a building and storey (`#20`, `#21`), a wall
/// (`#23`) and two systems (`#30`, `#31`), in `schema`.
fn model(schema: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #20=IFCBUILDING('3YvctVUKr0kugbFTf53O02',#5,'B',$,$,$,$,$,.ELEMENT.,$,$,$);\n\
         #21=IFCBUILDINGSTOREY('3YvctVUKr0kugbFTf53O00',#5,'L1',$,$,$,$,$,.ELEMENT.,$);\n\
         #23=IFCWALL('3YvctVUKr0kugbFTf53O05',#5,'W',$,$,$,$,$);\n\
         #30=IFCSYSTEM('3YvctVUKr0kugbFTf53O03',#5,'Heating',$,$);\n\
         #31=IFCSYSTEM('3YvctVUKr0kugbFTf53O04',#5,'Cooling',$,$);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    format!("2YvctVUKr0kugbFTf53O0{}", DIGITS[n] as char)
}

/// Run one call on a fresh transaction; assert it staged nothing and
/// return its error.
fn refused(
    schema: &str,
    model: &Model,
    system: EntityId,
    buildings: &[EntityId],
    owner: Option<EntityId>,
) -> SpatialAuthoringError {
    let mut tx = Transaction::new(model);
    let result = match owner {
        Some(owner) => {
            serve_buildings_with_owner_history(&mut tx, model, &guid(5), system, buildings, owner)
        }
        None => serve_buildings(&mut tx, model, &guid(5), system, buildings),
    };
    assert!(tx.edits().is_empty(), "{schema}: staged on refusal");
    result.expect_err("refused")
}

/// Written in each release, re-read through STEP text: the system and the
/// buildings land in their slots.
#[test]
fn a_system_serving_buildings_round_trips_in_every_release() {
    for (schema, _) in RELEASES {
        let mut model = model(schema);
        let mut tx = Transaction::new(&model);
        let relation = serve_buildings_with_owner_history(
            &mut tx,
            &model,
            &guid(1),
            SYSTEM,
            &[BUILDING, STOREY],
            OWNER,
        )
        .unwrap_or_else(|error| panic!("{schema}: {error}"));
        tx.commit(&mut model).unwrap();
        let back = StepCodec
            .read_bytes(&StepCodec.write_bytes(&model).unwrap())
            .unwrap();
        let record = back.get(relation).expect("written");
        assert_eq!(record.type_name.as_ref(), "IFCRELSERVICESBUILDINGS");
        assert_eq!(record.attributes.len(), 6, "{schema}");
        assert_eq!(record.attributes[1], Value::Ref(OWNER));
        assert_eq!(record.attributes[4], Value::Ref(SYSTEM));
        assert_eq!(
            record.attributes[5],
            Value::List(vec![Value::Ref(BUILDING), Value::Ref(STOREY)])
        );
    }
}

/// Every refusal, in every release, stages nothing.
#[test]
fn writer_refusals_stage_nothing() {
    for (schema, version) in RELEASES {
        let mut model = model(schema);
        for owner in [Some(OWNER), None] {
            // In IFC2X3 the plain writer refuses OwnerHistory first; the
            // target checks there are covered through the owner variant.
            if owner.is_none() && version == SchemaVersion::Ifc2x3 {
                continue;
            }
            // Relating end not a system.
            assert!(
                matches!(
                    refused(schema, &model, STOREY, &[BUILDING], owner),
                    SpatialAuthoringError::WrongReferenceType {
                        attribute: "RelatingSystem", target, ref actual, ..
                    } if target == STOREY && actual == "IFCBUILDINGSTOREY"
                ),
                "{schema}"
            );
            // Served end not spatial: a system and a wall.
            for other in [OTHER_SYSTEM, WALL] {
                assert!(
                    matches!(
                        refused(schema, &model, SYSTEM, &[BUILDING, other], owner),
                        SpatialAuthoringError::WrongReferenceType {
                            attribute: "RelatedBuildings", target, ..
                        } if target == other
                    ),
                    "{schema}"
                );
            }
            // Missing system and building.
            assert!(
                matches!(
                    refused(schema, &model, EntityId(98), &[BUILDING], owner),
                    SpatialAuthoringError::MissingReference {
                        attribute: "RelatingSystem",
                        ..
                    }
                ),
                "{schema}"
            );
            assert!(
                matches!(
                    refused(schema, &model, SYSTEM, &[EntityId(99)], owner),
                    SpatialAuthoringError::MissingReference {
                        attribute: "RelatedBuildings",
                        ..
                    }
                ),
                "{schema}"
            );
            // Empty and repeating sets, and the system among its buildings.
            for buildings in [&[][..], &[BUILDING, BUILDING], &[BUILDING, SYSTEM]] {
                assert!(
                    matches!(
                        refused(schema, &model, SYSTEM, buildings, owner),
                        SpatialAuthoringError::Invalid { .. }
                    ),
                    "{schema}: {buildings:?}"
                );
            }
        }
        // Without an owner history: IFC2X3 requires one.
        if version == SchemaVersion::Ifc2x3 {
            assert!(matches!(
                refused(schema, &model, SYSTEM, &[BUILDING], None),
                SpatialAuthoringError::AuthoringRequired {
                    attribute: "OwnerHistory",
                    ..
                }
            ));
        } else {
            let mut tx = Transaction::new(&model);
            serve_buildings(&mut tx, &model, &guid(6), SYSTEM, &[BUILDING]).unwrap();
            // ServicesBuildings is SET [0:1]: a second staged one is refused.
            let second = serve_buildings(&mut tx, &model, &guid(7), SYSTEM, &[STOREY]);
            assert!(
                matches!(
                    second,
                    Err(SpatialAuthoringError::Invalid {
                        attribute: "RelatingSystem",
                        ..
                    })
                ),
                "{schema}: {second:?}"
            );
            assert_eq!(tx.edits().len(), 1, "{schema}");
        }
        // A second one against the committed model is refused too.
        let mut tx = Transaction::new(&model);
        serve_buildings_with_owner_history(&mut tx, &model, &guid(8), SYSTEM, &[BUILDING], OWNER)
            .unwrap();
        tx.commit(&mut model).unwrap();
        assert!(
            matches!(
                refused(schema, &model, SYSTEM, &[STOREY], Some(OWNER)),
                SpatialAuthoringError::Invalid {
                    attribute: "RelatingSystem",
                    ..
                }
            ),
            "{schema}"
        );
        // Another system may still service the same building.
        let mut tx = Transaction::new(&model);
        serve_buildings_with_owner_history(
            &mut tx,
            &model,
            &guid(9),
            OTHER_SYSTEM,
            &[BUILDING],
            OWNER,
        )
        .unwrap();
    }
}

/// `RelatedBuildings` follows the declared release's table: an
/// `IfcSpatialZone` is an `IfcSpatialElement` (IFC4, IFC4X3) but not an
/// `IfcSpatialStructureElement` (IFC2X3, which declares no spatial zone).
#[test]
fn related_buildings_follow_the_release() {
    for (schema, version) in RELEASES {
        let model = model(schema);
        let mut tx = Transaction::new(&model);
        let zone = tx.create(Entity::new("IFCSPATIALZONE", vec![Value::Null; 10]));
        let staged = tx.edits().len();
        let result =
            serve_buildings_with_owner_history(&mut tx, &model, &guid(1), SYSTEM, &[zone], OWNER);
        if version == SchemaVersion::Ifc2x3 {
            match result {
                Err(SpatialAuthoringError::WrongReferenceType {
                    attribute: "RelatedBuildings",
                    target,
                    expected,
                    ..
                }) => {
                    assert_eq!(target, zone);
                    assert!(
                        expected.eq_ignore_ascii_case("IfcSpatialStructureElement"),
                        "{expected}"
                    );
                }
                other => panic!("{schema}: expected WrongReferenceType, got {other:?}"),
            }
            assert_eq!(tx.edits().len(), staged, "{schema}: staged on refusal");
        } else {
            result.unwrap_or_else(|error| panic!("{schema}: {error}"));
        }
    }
}

/// A header binding no single verified release is refused with the typed
/// error, before any target is looked at.
#[test]
fn unverified_releases_are_refused() {
    for token in ["IFC4X1", "IFC4X2"] {
        let mut model = model("IFC4");
        model.header_mut().schema = vec![token.to_owned()];
        for owner in [Some(OWNER), None] {
            assert!(
                matches!(
                    refused(token, &model, SYSTEM, &[BUILDING], owner),
                    SpatialAuthoringError::UnsupportedSchema { ref schema } if schema == token
                ),
                "{token}"
            );
        }
    }
    let mut model = model("IFC4");
    model.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    assert!(matches!(
        refused("two", &model, SYSTEM, &[BUILDING], Some(OWNER)),
        SpatialAuthoringError::MultipleSchemas { schemas: 2 }
    ));
}

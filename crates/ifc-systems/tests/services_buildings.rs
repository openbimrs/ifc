//! What a system serves (#230): `IfcRelServicesBuildings` in every verified
//! release, and the IFC4X3 `IfcSystem.ServicesFacilities` references.
//!
//! From the EXPRESS sources: `RelatingSystem : IfcSystem` and
//! `RelatedBuildings : SET [1:?] OF IfcSpatialStructureElement` (IFC2X3 TC1)
//! or `OF IfcSpatialElement` (IFC4 ADD2 TC1, IFC4X3 ADD2); the inverse
//! `IfcSystem.ServicesBuildings` is `SET [0:1]`. Only IFC4X3 declares
//! `ServicesFacilities : SET [0:?] OF IfcRelReferencedInSpatialStructure FOR
//! RelatedElements`, whose `RelatedElements` there is an
//! `IfcSpatialReferenceSelect` (IFC4: `IfcProduct`).

use ifc_model::{Codec, EntityId, Model, Transaction};
use ifc_step::StepCodec;
use ifc_systems::{
    create_system_with_owner_history, reference_in_spatial_structure_with_owner_history,
    serve_buildings, serve_buildings_with_owner_history, systems, SchemaVersion, SystemAnomaly,
    SystemAuthoringError,
};

const OWNER: EntityId = EntityId(5);
const BUILDING: EntityId = EntityId(20);
const STOREY: EntityId = EntityId(21);
const SPACE: EntityId = EntityId(22);
const SYSTEM: EntityId = EntityId(30);
const OTHER_SYSTEM: EntityId = EntityId(31);

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`), a building, storey and space (`#20`-`#22`), two
/// systems (`#30`, `#31`), and `extra` data lines, in `schema`.
fn model(schema: &str, version: SchemaVersion, extra: &str) -> Model {
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
         #20=IFCBUILDING('3YvctVUKr0kugbFTf53O02',#5,'B',$,$,$,$,$,.ELEMENT.,$,$,$);\n\
         #21=IFCBUILDINGSTOREY('3YvctVUKr0kugbFTf53O00',#5,'L1',$,$,$,$,$,.ELEMENT.,$);\n\
         #22=IFCSPACE('3YvctVUKr0kugbFTf53O01',#5,'R1',$,$,$,$,$,.ELEMENT.,{interior},$);\n\
         #30=IFCSYSTEM('3YvctVUKr0kugbFTf53O03',#5,'Heating',$,$);\n\
         #31=IFCSYSTEM('3YvctVUKr0kugbFTf53O04',#5,'Cooling',$,$);\n\
         {extra}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    format!("2YvctVUKr0kugbFTf53O0{}", DIGITS[n] as char)
}

fn round_trip(model: &Model) -> Model {
    let bytes = StepCodec.write_bytes(model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

fn system(model: &Model, id: EntityId) -> ifc_systems::System {
    let (found, _) = systems(model).unwrap();
    found.into_iter().find(|s| s.id == id).expect("system")
}

/// Written through the writer in each release, re-read through STEP text,
/// and read back onto the system. Only IFC4X3 reads the facilities view.
#[test]
fn serviced_buildings_round_trip_per_release() {
    for (schema, version) in RELEASES {
        let mut model = model(schema, version, "");
        let mut tx = Transaction::new(&model);
        let m = &model;
        let new_system =
            create_system_with_owner_history(&mut tx, m, &guid(1), Some("Vent"), OWNER).unwrap();
        let relation = serve_buildings_with_owner_history(
            &mut tx,
            m,
            &guid(2),
            new_system,
            &[STOREY, BUILDING],
            OWNER,
        )
        .unwrap();
        if version == SchemaVersion::Ifc4x3 {
            reference_in_spatial_structure_with_owner_history(
                &mut tx,
                m,
                &guid(3),
                BUILDING,
                &[new_system],
                OWNER,
            )
            .unwrap();
        }
        tx.commit(&mut model).expect("commit");
        let back = round_trip(&model);
        assert_eq!(
            back.get(relation).unwrap().attributes.len(),
            6,
            "{schema}: IfcRelServicesBuildings has six attributes"
        );
        let (_, anomalies) = systems(&back).unwrap();
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        let read = system(&back, new_system);
        assert_eq!(read.serviced_buildings, vec![STOREY, BUILDING], "{schema}");
        let facilities = if version == SchemaVersion::Ifc4x3 {
            vec![BUILDING]
        } else {
            vec![]
        };
        assert_eq!(read.serviced_facilities, facilities, "{schema}");
        assert!(system(&back, SYSTEM).serviced_buildings.is_empty());
    }
}

/// Dangling targets, a second relationship for one system, and a target the
/// release does not admit are reported, never read, in every release.
#[test]
fn reader_reports_anomalies_per_release() {
    let extra = "#40=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O10',#5,$,$,#30,(#20,#99,#31));\n\
                 #41=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O11',#5,$,$,#30,(#21));\n\
                 #42=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O12',#5,$,$,#98,(#21));\n\
                 #43=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O13',#5,$,$,#21,(#22));\n\
                 #44=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O14',#5,$,$,#31,(#22));\n";
    for (schema, version) in RELEASES {
        let model = round_trip(&model(schema, version, extra));
        let (found, anomalies) = systems(&model).unwrap();
        let get = |id| found.iter().find(|s| s.id == id).unwrap();
        assert_eq!(get(SYSTEM).serviced_buildings, vec![BUILDING], "{schema}");
        assert_eq!(
            get(OTHER_SYSTEM).serviced_buildings,
            vec![SPACE],
            "{schema}"
        );
        let expected = vec![
            SystemAnomaly::Dangling {
                relation: EntityId(40),
                missing: EntityId(99),
            },
            SystemAnomaly::ServicedNotSpatial {
                relation: EntityId(40),
                target: OTHER_SYSTEM,
                type_name: "IFCSYSTEM".into(),
            },
            SystemAnomaly::ServicesBuildingsTwice {
                system: SYSTEM,
                kept: EntityId(40),
                rejected: EntityId(41),
            },
            SystemAnomaly::Dangling {
                relation: EntityId(42),
                missing: EntityId(98),
            },
            SystemAnomaly::NotASystem {
                relation: EntityId(43),
                group: STOREY,
                type_name: "IFCBUILDINGSTOREY".into(),
            },
        ];
        assert_eq!(anomalies, expected, "{schema}");
    }
}

/// The admissible target comes from the release: IFC4 admits an
/// `IfcSpatialZone` (an `IfcSpatialElement`), which IFC2X3 does not declare.
#[test]
fn ifc4_admits_a_spatial_zone() {
    let extra = "#50=IFCSPATIALZONE('2YvctVUKr0kugbFTf53O20',#5,'Z',$,$,$,$,$,.THERMAL.);\n\
                 #51=IFCRELSERVICESBUILDINGS('2YvctVUKr0kugbFTf53O21',#5,$,$,#30,(#50));\n";
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let model = round_trip(&model(schema, version, extra));
        let (_, anomalies) = systems(&model).unwrap();
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        assert_eq!(
            system(&model, SYSTEM).serviced_buildings,
            vec![EntityId(50)]
        );
    }
}

/// IFC4 cannot reference a system in a spatial structure (`RelatedElements`
/// is `IfcProduct`), so no facilities view is read there; IFC4X3 reads it
/// and reports a dangling structure.
#[test]
fn facilities_are_ifc4x3_only() {
    let extra =
        "#60=IFCRELREFERENCEDINSPATIALSTRUCTURE('2YvctVUKr0kugbFTf53O30',#5,$,$,(#30),#20);\n\
         #61=IFCRELREFERENCEDINSPATIALSTRUCTURE('2YvctVUKr0kugbFTf53O31',#5,$,$,(#30,#22),#97);\n\
         #62=IFCRELREFERENCEDINSPATIALSTRUCTURE('2YvctVUKr0kugbFTf53O32',#5,$,$,(#30),#21);\n";
    let ifc4 = round_trip(&model("IFC4", SchemaVersion::Ifc4, extra));
    let (_, anomalies) = systems(&ifc4).unwrap();
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert!(system(&ifc4, SYSTEM).serviced_facilities.is_empty());

    let x3 = round_trip(&model("IFC4X3_ADD2", SchemaVersion::Ifc4x3, extra));
    let (_, anomalies) = systems(&x3).unwrap();
    assert_eq!(
        anomalies,
        vec![SystemAnomaly::Dangling {
            relation: EntityId(61),
            missing: EntityId(97),
        }]
    );
    assert_eq!(
        system(&x3, SYSTEM).serviced_facilities,
        vec![BUILDING, STOREY]
    );
}

/// Every refusal stages nothing.
#[test]
fn writer_refusals_stage_nothing() {
    for (schema, version) in RELEASES {
        let mut model = model(schema, version, "");
        let refused = |model: &Model,
                       system: EntityId,
                       buildings: &[EntityId],
                       owner: Option<EntityId>|
         -> SystemAuthoringError {
            let mut tx = Transaction::new(model);
            let result = match owner {
                Some(owner) => serve_buildings_with_owner_history(
                    &mut tx,
                    model,
                    &guid(5),
                    system,
                    buildings,
                    owner,
                ),
                None => serve_buildings(&mut tx, model, &guid(5), system, buildings),
            };
            assert!(tx.edits().is_empty(), "{schema}: staged on refusal");
            result.expect_err("refused")
        };
        // Relating end not a system.
        assert!(matches!(
            refused(&model, STOREY, &[BUILDING], Some(OWNER)),
            SystemAuthoringError::WrongReferenceType { attribute: "RelatingSystem", target, .. }
                if target == STOREY
        ));
        // Served end not spatial.
        assert!(matches!(
            refused(&model, SYSTEM, &[BUILDING, OTHER_SYSTEM], Some(OWNER)),
            SystemAuthoringError::WrongReferenceType { attribute: "RelatedBuildings", target, ref actual, .. }
                if target == OTHER_SYSTEM && actual == "IFCSYSTEM"
        ));
        // Missing building.
        assert!(matches!(
            refused(&model, SYSTEM, &[EntityId(99)], Some(OWNER)),
            SystemAuthoringError::MissingReference {
                attribute: "RelatedBuildings",
                ..
            }
        ));
        // Empty and repeating sets.
        assert!(matches!(
            refused(&model, SYSTEM, &[], Some(OWNER)),
            SystemAuthoringError::Invalid { .. }
        ));
        assert!(matches!(
            refused(&model, SYSTEM, &[BUILDING, BUILDING], Some(OWNER)),
            SystemAuthoringError::Invalid { .. }
        ));
        // Without an owner history: IFC2X3 requires one.
        if version == SchemaVersion::Ifc2x3 {
            assert!(matches!(
                refused(&model, SYSTEM, &[BUILDING], None),
                SystemAuthoringError::AuthoringRequired {
                    attribute: "OwnerHistory",
                    ..
                }
            ));
        } else {
            let mut tx = Transaction::new(&model);
            serve_buildings(&mut tx, &model, &guid(6), SYSTEM, &[BUILDING]).unwrap();
            // ServicesBuildings is SET [0:1]: a second staged one is refused.
            let second = serve_buildings(&mut tx, &model, &guid(7), SYSTEM, &[STOREY]);
            assert!(matches!(
                second,
                Err(SystemAuthoringError::Invalid {
                    attribute: "RelatingSystem",
                    ..
                })
            ));
            assert_eq!(tx.edits().len(), 1, "{schema}");
        }
        // A second one against the committed model is refused too.
        let mut tx = Transaction::new(&model);
        serve_buildings_with_owner_history(&mut tx, &model, &guid(8), SYSTEM, &[BUILDING], OWNER)
            .unwrap();
        tx.commit(&mut model).unwrap();
        assert!(matches!(
            refused(&model, SYSTEM, &[STOREY], Some(OWNER)),
            SystemAuthoringError::Invalid {
                attribute: "RelatingSystem",
                ..
            }
        ));
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

/// A header binding no verified release is refused with the typed error.
#[test]
fn unverified_releases_are_refused() {
    for token in ["IFC4X1", "IFC4X2"] {
        let mut model = model("IFC4", SchemaVersion::Ifc4, "");
        model.header_mut().schema = vec![token.to_owned()];
        let mut tx = Transaction::new(&model);
        let refused = serve_buildings(&mut tx, &model, &guid(1), SYSTEM, &[BUILDING]);
        assert!(
            matches!(refused, Err(SystemAuthoringError::UnsupportedSchema { ref schema }) if schema == token),
            "{token}: {refused:?}"
        );
        assert!(tx.edits().is_empty());
    }
}

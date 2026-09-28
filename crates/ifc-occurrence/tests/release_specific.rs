//! #214: what IFC2X3 requires of a few occurrence classes, and the type
//! pairing each release states.
//!
//! From the EXPRESS sources: IFC2X3 TC1 requires `ShapeType` on `IfcRamp`,
//! `IfcRoof` and `IfcStair`, `NominalDiameter` and `CrossSectionArea` on
//! `IfcReinforcingBar` (also `BarRole`) and `IfcTendon`, and the six bar
//! measures on `IfcReinforcingMesh`; IFC4 ADD2 TC1 and IFC4X3 ADD2 declare
//! the measures `OPTIONAL` and have no `ShapeType` or `BarRole`. IFC4 pairs
//! `IfcDoor` with `IfcDoorType` (`CorrectStyleAssigned`); IFC2X3 states no
//! rule and types a door by an `IfcDoorStyle`.

use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_occurrence::table::{
    Occurrence, ALL, IFCBEAM, IFCDOOR, IFCRAMP, IFCREINFORCINGBAR, IFCREINFORCINGMESH, IFCROOF,
    IFCSTAIR, IFCTENDON, IFCWALL, IFCWINDOW,
};
use ifc_occurrence::{create_with_owner_history, MeshBars, OccurrenceDraft, OccurrenceError};
use ifc_schema::{for_version, Schema, SchemaVersion};
use ifc_step::StepCodec;

const OWNER: EntityId = EntityId(5);
const DOOR_STYLE: EntityId = EntityId(12);
const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

/// Actors and an owner history (`#5`) and a door style (`#12`, IFC2X3 and
/// IFC4 both declare `IfcDoorStyle` with this layout) in `schema`.
fn model(schema: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #12=IFCDOORSTYLE('2xS3BCk291UvhgP2dvNsgq',#5,'DS',$,$,$,$,$,\
         .SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn table(version: SchemaVersion) -> &'static Schema {
    for_version(version).expect("bundled")
}

fn slot(schema: &Schema, entity: &str, attribute: &str) -> usize {
    schema
        .attribute_names(entity)
        .iter()
        .position(|name| *name == attribute)
        .unwrap_or_else(|| panic!("{entity}.{attribute}"))
}

fn author(
    model: &Model,
    kind: Occurrence,
    token: Option<&str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'_>,
) -> (Transaction, Result<EntityId, OccurrenceError>) {
    let mut tx = Transaction::new(model);
    let result =
        create_with_owner_history(&mut tx, model, kind, GUID, token, typed_by, draft, OWNER);
    if result.is_err() {
        assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    }
    (tx, result)
}

fn refused(
    model: &Model,
    kind: Occurrence,
    token: Option<&str>,
    typed_by: Option<EntityId>,
    draft: OccurrenceDraft<'_>,
) -> OccurrenceError {
    author(model, kind, token, typed_by, draft)
        .1
        .expect_err("refused")
}

/// A class, its token, the draft, and the values expected back by name.
type Case<'a> = (
    Occurrence,
    Option<&'a str>,
    OccurrenceDraft<'a>,
    &'a [(&'a str, Value)],
);

/// The IFC2X3 records the draft could not carry before are written, and
/// read back through `ifc-step` with the values in the release's slots.
#[test]
fn ifc2x3_records_round_trip_with_their_required_attributes() {
    let bars = MeshBars::new(0.008, 5.0e-5, 0.15);
    let cases: [Case<'_>; 6] = [
        (
            IFCRAMP,
            None,
            OccurrenceDraft::new().shape_type("STRAIGHT_RUN_RAMP"),
            &[("ShapeType", Value::Enum("STRAIGHT_RUN_RAMP".into()))],
        ),
        (
            IFCROOF,
            None,
            OccurrenceDraft::new().shape_type("FLAT_ROOF"),
            &[("ShapeType", Value::Enum("FLAT_ROOF".into()))],
        ),
        (
            IFCSTAIR,
            None,
            OccurrenceDraft::new().shape_type("STRAIGHT_RUN_STAIR"),
            &[("ShapeType", Value::Enum("STRAIGHT_RUN_STAIR".into()))],
        ),
        (
            IFCREINFORCINGBAR,
            None,
            OccurrenceDraft::new()
                .nominal_diameter(0.012)
                .cross_section_area(1.13e-4)
                .bar_role("MAIN"),
            &[
                ("NominalDiameter", Value::Real(0.012)),
                ("CrossSectionArea", Value::Real(1.13e-4)),
                ("BarRole", Value::Enum("MAIN".into())),
            ],
        ),
        (
            IFCTENDON,
            Some("STRAND"),
            OccurrenceDraft::new()
                .nominal_diameter(0.015)
                .cross_section_area(1.4e-4),
            &[
                ("NominalDiameter", Value::Real(0.015)),
                ("CrossSectionArea", Value::Real(1.4e-4)),
                ("PredefinedType", Value::Enum("STRAND".into())),
            ],
        ),
        (
            IFCREINFORCINGMESH,
            None,
            OccurrenceDraft::new()
                .longitudinal_bars(bars)
                .transverse_bars(MeshBars::new(0.006, 2.8e-5, 0.2)),
            &[
                ("LongitudinalBarNominalDiameter", Value::Real(0.008)),
                ("LongitudinalBarCrossSectionArea", Value::Real(5.0e-5)),
                ("LongitudinalBarSpacing", Value::Real(0.15)),
                ("TransverseBarNominalDiameter", Value::Real(0.006)),
                ("TransverseBarCrossSectionArea", Value::Real(2.8e-5)),
                ("TransverseBarSpacing", Value::Real(0.2)),
            ],
        ),
    ];
    let schema = table(SchemaVersion::Ifc2x3);
    for (kind, token, draft, expected) in cases {
        let mut model = model("IFC2X3");
        let without = refused(&model, kind, token, None, OccurrenceDraft::new());
        assert!(
            matches!(without, OccurrenceError::AuthoringRequired { .. }),
            "{}: {without:?}",
            kind.type_name
        );
        let (tx, result) = author(&model, kind, token, None, draft);
        let id = result.unwrap_or_else(|error| panic!("{}: {error:?}", kind.type_name));
        tx.commit(&mut model).expect("commit");
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let record = back.get(id).expect("read back");
        assert_eq!(record.type_name.as_ref(), kind.type_name);
        assert_eq!(
            record.attributes.len(),
            schema.attributes(kind.type_name).len()
        );
        for (attribute, value) in expected {
            assert_eq!(
                &record.attributes[slot(schema, kind.type_name, attribute)],
                value,
                "{}.{attribute}",
                kind.type_name
            );
        }
    }
}

/// The reinforcement measures are `OPTIONAL` in IFC4 and IFC4X3 and are
/// written there when given; `ShapeType` and `BarRole` are refused there.
#[test]
fn later_releases_write_the_measures_and_refuse_what_they_lack() {
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let model = model(schema);
        let (tx, result) = author(
            &model,
            IFCREINFORCINGBAR,
            None,
            None,
            OccurrenceDraft::new().nominal_diameter(0.012),
        );
        result.expect(schema);
        let ifc_model::Edit::Create { entity, .. } = &tx.edits()[0] else {
            panic!("a create");
        };
        let at = slot(table(version), "IFCREINFORCINGBAR", "NominalDiameter");
        assert_eq!(entity.attributes[at], Value::Real(0.012));

        for (kind, draft, attribute) in [
            (
                IFCSTAIR,
                OccurrenceDraft::new().shape_type("STRAIGHT_RUN_STAIR"),
                "ShapeType",
            ),
            (
                IFCREINFORCINGBAR,
                OccurrenceDraft::new().bar_role("MAIN"),
                "BarRole",
            ),
            (
                IFCWALL,
                OccurrenceDraft::new().nominal_diameter(0.01),
                "NominalDiameter",
            ),
        ] {
            assert_eq!(
                refused(&model, kind, None, None, draft),
                OccurrenceError::AuthoringNotInSchema {
                    entity: kind.type_name,
                    attribute,
                    schema: version,
                },
                "{schema}"
            );
        }
    }
}

#[test]
fn a_token_outside_the_release_enumeration_is_refused() {
    let model = model("IFC2X3");
    assert_eq!(
        refused(
            &model,
            IFCSTAIR,
            None,
            None,
            OccurrenceDraft::new().shape_type("FLAT_ROOF"),
        ),
        OccurrenceError::UnknownToken {
            entity: "IFCSTAIR",
            attribute: "ShapeType",
            token: "FLAT_ROOF".into(),
        }
    );
}

#[test]
fn a_measure_the_type_does_not_admit_is_refused() {
    let model = model("IFC2X3");
    for (draft, attribute) in [
        (
            OccurrenceDraft::new()
                .nominal_diameter(0.0)
                .cross_section_area(1.0e-4)
                .bar_role("MAIN"),
            "NominalDiameter",
        ),
        (
            OccurrenceDraft::new()
                .nominal_diameter(-0.01)
                .cross_section_area(1.0e-4)
                .bar_role("MAIN"),
            "NominalDiameter",
        ),
        (
            OccurrenceDraft::new()
                .nominal_diameter(0.01)
                .cross_section_area(f64::NAN)
                .bar_role("MAIN"),
            "CrossSectionArea",
        ),
    ] {
        let error = refused(&model, IFCREINFORCINGBAR, None, None, draft);
        assert!(
            matches!(
                &error,
                OccurrenceError::InvalidMeasure { entity: "IFCREINFORCINGBAR", attribute: a, .. }
                    if *a == attribute
            ),
            "{error:?}"
        );
    }
    let bad_spacing = MeshBars::new(0.008, 5.0e-5, f64::INFINITY);
    let error = refused(
        &model,
        IFCREINFORCINGMESH,
        None,
        None,
        OccurrenceDraft::new()
            .longitudinal_bars(bad_spacing)
            .transverse_bars(bad_spacing),
    );
    assert!(
        matches!(
            error,
            OccurrenceError::InvalidMeasure {
                attribute: "LongitudinalBarSpacing",
                ..
            }
        ),
        "{error:?}"
    );
}

/// IFC2X3 types a door by an `IfcDoorStyle`; IFC4 requires `IfcDoorType`.
#[test]
fn the_door_pairing_is_the_releases_own() {
    let mut ifc2x3 = model("IFC2X3");
    let (tx, result) = author(
        &ifc2x3,
        IFCDOOR,
        None,
        Some(DOOR_STYLE),
        OccurrenceDraft::new(),
    );
    result.expect("an IFC2X3 door takes an IfcDoorStyle");
    tx.commit(&mut ifc2x3).expect("commit");

    let ifc4 = model("IFC4");
    assert_eq!(
        refused(
            &ifc4,
            IFCDOOR,
            None,
            Some(DOOR_STYLE),
            OccurrenceDraft::new()
        ),
        OccurrenceError::WrongTypeClass {
            entity: "IFCDOOR",
            expected: "IFCDOORTYPE",
            found: "IFCDOORSTYLE".into(),
        }
    );
    // A class whose type IFC2X3 does not declare cannot be typed there.
    assert_eq!(
        refused(
            &ifc2x3,
            IFCSTAIR,
            None,
            Some(DOOR_STYLE),
            OccurrenceDraft::new().shape_type("STRAIGHT_RUN_STAIR"),
        ),
        OccurrenceError::TypeClassNotInSchema {
            entity: "IFCSTAIR",
            schema: SchemaVersion::Ifc2x3,
        }
    );
    // An IFC2X3 beam still takes the beam type, not a door style.
    assert!(matches!(
        refused(
            &ifc2x3,
            IFCBEAM,
            None,
            Some(DOOR_STYLE),
            OccurrenceDraft::new()
        ),
        OccurrenceError::WrongTypeClass {
            expected: "IFCBEAMTYPE",
            ..
        }
    ));
}

/// Every pairing column names a concrete `IfcTypeObject` subtype of its
/// release, and only for a class that release declares; IFC2X3 doors and
/// windows are paired with their styles.
#[test]
fn every_pairing_names_a_type_of_its_release() {
    let mut paired = [0; 3];
    for kind in ALL {
        for (index, (version, class)) in [
            (SchemaVersion::Ifc2x3, kind.ifc2x3_type_class),
            (SchemaVersion::Ifc4, kind.ifc4_type_class),
            (SchemaVersion::Ifc4x3, kind.type_class),
        ]
        .into_iter()
        .enumerate()
        {
            let Some(class) = class else { continue };
            let schema = table(version);
            assert!(
                schema.entity(kind.type_name).is_some_and(|e| !e.abstract_),
                "{version:?}: {} paired but not declared",
                kind.type_name
            );
            // IFC4 ADD2 TC1's own erratum: `IfcTransformer.CorrectTypeAssigned`
            // names 'IFC4.IFCTRANFORMERTYPE', a class IFC4 does not declare.
            // The column records the rule as written, so an IFC4 transformer
            // cannot be typed; it is not silently corrected.
            if (version, kind.type_name) == (SchemaVersion::Ifc4, "IFCTRANSFORMER") {
                assert_eq!(class, "IFCTRANFORMERTYPE");
                assert!(schema.entity(class).is_none());
                continue;
            }
            assert!(
                schema.entity(class).is_some_and(|e| !e.abstract_)
                    && schema.is_a(class, "IFCTYPEOBJECT"),
                "{version:?}: {} paired with {class}",
                kind.type_name
            );
            paired[index] += 1;
        }
    }
    assert_eq!(IFCDOOR.ifc2x3_type_class, Some("IFCDOORSTYLE"));
    assert_eq!(IFCWINDOW.ifc2x3_type_class, Some("IFCWINDOWSTYLE"));
    assert_eq!(IFCDOOR.ifc4_type_class, Some("IFCDOORTYPE"));
    assert_eq!(IFCSTAIR.ifc2x3_type_class, None);
    assert!(
        paired[0] >= 15 && paired[1] >= 90 && paired[2] >= 100,
        "{paired:?}"
    );
}

//! Exact property and unit resolution on IFC4X3 ADD2 models (#76).
//!
//! Every model is parsed from STEP text under `FILE_SCHEMA(('IFC4X3_ADD2'))`
//! and resolved against the bundled IFC4X3 ADD2 table. The facts pinned here
//! come from `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`:
//!
//! - `IfcRelDefinesByProperties.RelatedObjects : SET [1:?] OF
//!   IfcObjectDefinition`, with `NoRelatedTypeObject` (as IFC4; IFC2X3
//!   inherits `SET OF IfcObject`). `IfcRelDefinesByType.RelatedObjects` is
//!   `SET [1:?] OF IfcObject`.
//! - `RelatingPropertyDefinition : IfcPropertySetDefinitionSelect`, whose
//!   `IfcPropertySetDefinitionSet = SET [1:?] OF IfcPropertySetDefinition`.
//! - `IfcTypeObject.HasPropertySets : OPTIONAL SET [1:?] OF
//!   IfcPropertySetDefinition` at slot 5.
//! - `IfcQuantityNumber` (`NumberValue : IfcNumericMeasure`) is new in
//!   IFC4X3; IFC4 does not declare it.
//! - `IfcDerivedUnit` gains `Name : OPTIONAL IfcLabel`, so it has four
//!   attributes where IFC4 has three.
//! - `IfcDimensionsForSIUnit` and `IfcCorrectDimensions` give the farad and
//!   capacitance IFC4's `(-2, -1, 4, 2, 0, 0, 0)`.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_properties, exact_property, exact_schema, exact_unit, ExactProperty, ExactPropertyError,
    ExactResolution, ExactSource, ExactUnitError, ExactValue, SchemaVersion,
};
use ifc_step::StepCodec;

/// An `IfcWall` occurrence, IFC4X3 arity (9).
const WALL: &str = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";
/// `IfcPropertySingleValue` `FireRating` #10 in `Pset_WallCommon` #11.
const FIRE: &str = "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);";
const PSET: &str = "#11=IFCPROPERTYSET('0000000000000000000011',$,'Pset_WallCommon',$,(#10));";

fn step(schema: &str, records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn ifc4x3(records: &[&str]) -> Model {
    step("IFC4X3_ADD2", records)
}

/// `IfcRelDefinesByProperties` #`id` relating `objects` to `definition`.
fn defines(id: u64, objects: &[u64], definition: &str) -> String {
    let refs: Vec<String> = objects.iter().map(|o| format!("#{o}")).collect();
    format!(
        "#{id}=IFCRELDEFINESBYPROPERTIES('{id:0>22}',$,$,$,({}),{definition});",
        refs.join(",")
    )
}

fn present(result: Result<ExactResolution, ExactPropertyError>) -> ExactProperty {
    match result {
        Ok(ExactResolution::Present(property)) => property,
        other => panic!("expected a present property, got {other:?}"),
    }
}

#[test]
fn an_ifc4x3_add2_header_binds_the_ifc4x3_table() {
    let m = ifc4x3(&[WALL, FIRE, PSET, &defines(12, &[1], "#11")]);
    assert_eq!(exact_schema(&m), Ok(SchemaVersion::Ifc4x3));
    let property = present(exact_property(
        &m,
        EntityId(1),
        Some("Pset_WallCommon"),
        "FireRating",
    ));
    assert_eq!(property.source, ExactSource::Occurrence);
    assert_eq!(property.set_id, EntityId(11));
    assert_eq!(property.property_id, EntityId(10));
    assert_eq!(property.value_type.as_deref(), Some("IFCLABEL"));
    assert_eq!(property.value, ExactValue::Text("F90".into()));
    assert_eq!(
        exact_property(&m, EntityId(1), None, "LoadBearing"),
        Ok(ExactResolution::Absent)
    );
    // The same entry through the enumeration.
    let all = exact_properties(&m, EntityId(1)).expect("enumerates");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name.as_ref(), "FireRating");
    assert_eq!(all[0].property, property);
}

#[test]
fn records_are_held_to_ifc4x3_arities() {
    // An IFC2X3-arity wall (8 slots) in an IFC4X3 file.
    let m = ifc4x3(&[
        "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$);",
        FIRE,
        PSET,
        &defines(12, &[1], "#11"),
    ]);
    assert_eq!(
        exact_property(&m, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::MalformedEntitySlots {
            entity: EntityId(1),
            type_name: "IFCWALL".into(),
            expected: 9,
            actual: 8,
        })
    );
}

#[test]
fn related_objects_admit_object_definitions_but_never_type_objects() {
    // `IfcProject` is an `IfcContext`, an `IfcObjectDefinition` but not an
    // `IfcObject`: a legal occurrence target of property assignment.
    let project = ifc4x3(&[
        "#1=IFCPROJECT('0000000000000000000001',$,'P',$,$,$,$,$,$);",
        FIRE,
        PSET,
        &defines(12, &[1], "#11"),
    ]);
    assert_eq!(
        present(exact_property(&project, EntityId(1), None, "FireRating")).property_id,
        EntityId(10)
    );
    // `NoRelatedTypeObject`: a type object in `RelatedObjects` is refused.
    let wall_type = "#2=IFCWALLTYPE('0000000000000000000002',$,'WT',$,$,$,$,$,$,.STANDARD.);";
    let typed = ifc4x3(&[WALL, wall_type, FIRE, PSET, &defines(12, &[1, 2], "#11")]);
    assert_eq!(
        exact_property(&typed, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::InvalidOccurrenceTarget {
            relationship: EntityId(12),
            object: EntityId(2),
        })
    );
    // `IfcRelDefinesByType.RelatedObjects` is `SET OF IfcObject`: a
    // project cannot be typed.
    let typed_project = ifc4x3(&[
        "#1=IFCPROJECT('0000000000000000000001',$,'P',$,$,$,$,$,$);",
        wall_type,
        "#13=IFCRELDEFINESBYTYPE('0000000000000000000013',$,$,$,(#1),#2);",
    ]);
    assert_eq!(
        exact_property(&typed_project, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::InvalidTypeTarget {
            relationship: EntityId(13),
            object: EntityId(1),
        })
    );
}

#[test]
fn a_property_set_definition_set_is_traversed_and_held_to_its_set_rules() {
    let other = "#13=IFCPROPERTYSET('0000000000000000000013',$,'Pset_Other',$,(#14));";
    let load = "#14=IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN(.T.),$);";
    let m = ifc4x3(&[
        WALL,
        FIRE,
        PSET,
        other,
        load,
        &defines(12, &[1], "IFCPROPERTYSETDEFINITIONSET((#11,#13))"),
    ]);
    assert_eq!(
        present(exact_property(&m, EntityId(1), None, "FireRating")).set_id,
        EntityId(11)
    );
    let load_bearing = present(exact_property(&m, EntityId(1), None, "LoadBearing"));
    assert_eq!(load_bearing.set_id, EntityId(13));
    assert_eq!(load_bearing.value, ExactValue::Bool(true));

    let empty = ifc4x3(&[
        WALL,
        FIRE,
        PSET,
        &defines(12, &[1], "IFCPROPERTYSETDEFINITIONSET(())"),
    ]);
    assert_eq!(
        exact_property(&empty, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(12),
            attribute: "RelatingPropertyDefinition",
        })
    );
    let repeated = ifc4x3(&[
        WALL,
        FIRE,
        PSET,
        &defines(12, &[1], "IFCPROPERTYSETDEFINITIONSET((#11,#11))"),
    ]);
    assert_eq!(
        exact_property(&repeated, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::DuplicateAggregateMember {
            entity: EntityId(12),
            attribute: "RelatingPropertyDefinition",
            member: EntityId(11),
        })
    );
}

/// A wall typed by #2, whose `HasPropertySets` is `has_sets`.
fn typed_wall(has_sets: &str, extra: &[&str]) -> Model {
    let wall_type =
        format!("#2=IFCWALLTYPE('0000000000000000000002',$,'WT',$,$,{has_sets},$,$,$,.STANDARD.);");
    let mut records = vec![
        WALL,
        wall_type.as_str(),
        FIRE,
        PSET,
        "#13=IFCRELDEFINESBYTYPE('0000000000000000000013',$,$,$,(#1),#2);",
    ];
    records.extend_from_slice(extra);
    ifc4x3(&records)
}

#[test]
fn type_object_property_sets_are_optional_but_never_empty() {
    // Inherited through `HasPropertySets`.
    let inherited = typed_wall("(#11)", &[]);
    let property = present(exact_property(&inherited, EntityId(1), None, "FireRating"));
    assert_eq!(property.source, ExactSource::Type(EntityId(2)));
    // An occurrence value of the same set and name overrides it.
    let overridden = typed_wall(
        "(#11)",
        &[
            "#20=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F30'),$);",
            "#21=IFCPROPERTYSET('0000000000000000000021',$,'Pset_WallCommon',$,(#20));",
            &defines(22, &[1], "#21"),
        ],
    );
    let property = present(exact_property(&overridden, EntityId(1), None, "FireRating"));
    assert_eq!(property.source, ExactSource::Occurrence);
    assert_eq!(property.value, ExactValue::Text("F30".into()));
    // `$` states no sets: a proven absence.
    assert_eq!(
        exact_property(&typed_wall("$", &[]), EntityId(1), None, "FireRating"),
        Ok(ExactResolution::Absent)
    );
    // Present but empty breaks `SET [1:?]`: incomplete evidence.
    assert_eq!(
        exact_property(&typed_wall("()", &[]), EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(2),
            attribute: "HasPropertySets",
        })
    );
}

#[test]
fn a_quantity_number_resolves_only_where_the_release_declares_it() {
    let records = [
        WALL,
        "#10=IFCQUANTITYNUMBER('Openings',$,$,3.,$);",
        "#11=IFCELEMENTQUANTITY('0000000000000000000011',$,'Qto_Custom',$,$,(#10));",
    ];
    let mut with_rel: Vec<&str> = records.to_vec();
    let rel = defines(12, &[1], "#11");
    with_rel.push(&rel);
    let quantity = present(exact_property(
        &ifc4x3(&with_rel),
        EntityId(1),
        Some("Qto_Custom"),
        "Openings",
    ));
    assert_eq!(quantity.value_type.as_deref(), Some("IFCNUMERICMEASURE"));
    assert_eq!(quantity.value, ExactValue::Real(3.0));
    // IFC4 has no `IfcQuantityNumber`: the file mixes releases.
    assert!(matches!(
        exact_property(&step("IFC4", &with_rel), EntityId(1), None, "Openings"),
        Err(ExactPropertyError::NotInSchema {
            entity: EntityId(10),
            schema: SchemaVersion::Ifc4,
            ..
        })
    ));
}

#[test]
fn units_are_read_with_ifc4x3_tables_and_arities() {
    // The farad takes IFC4X3's own dimensions (IFC4's, not IFC2X3's).
    let farad = ifc4x3(&["#1=IFCSIUNIT(*,.ELECTRICCAPACITANCEUNIT.,.MICRO.,.FARAD.);"]);
    let unit = exact_unit(&farad, "IFCELECTRICCAPACITANCEMEASURE", Some(EntityId(1)))
        .expect("an IFC4X3 farad resolves");
    assert_eq!(unit.dimensions, [-2, -1, 4, 2, 0, 0, 0]);
    assert!((unit.scale - 1e-6).abs() < 1e-18);

    // `IfcDerivedUnit` has four attributes in IFC4X3 (`Name` is new).
    let derived = |unit: &str| {
        ifc4x3(&[
            "#1=IFCSIUNIT(*,.POWERUNIT.,$,.WATT.);",
            "#2=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
            "#3=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);",
            "#4=IFCDERIVEDUNITELEMENT(#1,1);",
            "#5=IFCDERIVEDUNITELEMENT(#2,-2);",
            "#6=IFCDERIVEDUNITELEMENT(#3,-1);",
            unit,
        ])
    };
    let named = derived("#7=IFCDERIVEDUNIT((#4,#5,#6),.THERMALTRANSMITTANCEUNIT.,$,'U');");
    let unit = exact_unit(&named, "IFCTHERMALTRANSMITTANCEMEASURE", Some(EntityId(7)))
        .expect("an IFC4X3 derived unit resolves");
    assert_eq!(unit.dimensions, [0, 1, -3, 0, -1, 0, 0]);
    let ifc4_arity = derived("#7=IFCDERIVEDUNIT((#4,#5,#6),.THERMALTRANSMITTANCEUNIT.,$);");
    assert_eq!(
        exact_unit(
            &ifc4_arity,
            "IFCTHERMALTRANSMITTANCEMEASURE",
            Some(EntityId(7))
        ),
        Err(ExactUnitError::Structure(
            ExactPropertyError::MalformedEntitySlots {
                entity: EntityId(7),
                type_name: "IFCDERIVEDUNIT".into(),
                expected: 4,
                actual: 3,
            }
        ))
    );
}

#[test]
fn an_ifc4x3_only_referent_resolves_its_stationing_in_the_alignment_fixture() {
    // `IfcReferent` #520 carries `Pset_Stationing` #533; neither IFC2X3 nor
    // IFC4 declares `IfcReferent`.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_alignment_layout.ifc");
    let m = StepCodec.read_path(&path).expect("fixture parses");
    assert_eq!(exact_schema(&m), Ok(SchemaVersion::Ifc4x3));
    let station = present(exact_property(
        &m,
        EntityId(520),
        Some("Pset_Stationing"),
        "Station",
    ));
    assert_eq!(station.set_id, EntityId(533));
    assert_eq!(station.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
    assert_eq!(station.value, ExactValue::Real(5.0));
    let names: Vec<_> = exact_properties(&m, EntityId(520))
        .expect("every stationing property is a single value")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert_eq!(
        names,
        ["IncomingStation", "Station", "HasIncreasingStation"].map(std::sync::Arc::<str>::from)
    );
}

#[test]
fn a_bare_ifc4x3_token_binds_the_same_release() {
    // `ifc_schema` maps both header tokens to the one bundled IFC4X3 ADD2
    // table; exact resolution follows that mapping and nothing else.
    let m = step("IFC4X3", &[WALL]);
    assert_eq!(exact_schema(&m), Ok(SchemaVersion::Ifc4x3));
    assert!(matches!(
        exact_schema(&step("IFC4X3_ADD1", &[WALL])),
        Err(ExactPropertyError::UnsupportedSchema { .. })
    ));
}

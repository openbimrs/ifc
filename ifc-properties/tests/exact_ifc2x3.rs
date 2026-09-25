//! Exact property resolution on IFC2X3 models (#48).
//!
//! Every model here is parsed from STEP text through the strict codec, so the
//! records carry IFC2X3 TC1 arities exactly as an exporter writes them: an
//! `IfcWall` has 8 slots (IFC4: 9), an `IfcDoor` 10 (IFC4: 13), and
//! `IfcRelDefinesByProperties.RelatingPropertyDefinition` is a plain
//! `IfcPropertySetDefinition` reference (IFC4 widens it to a select that
//! admits `IfcPropertySetDefinitionSet`). The resolver must bind to the
//! release the header declares and refuse anything foreign to it.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_property, exact_schema, ExactLogical, ExactPropertyError, ExactResolution, ExactSource,
    ExactValue, SchemaVersion,
};
use ifc_step::StepCodec;

/// An `IfcWall` occurrence, IFC2X3 arity (8).
const WALL: &str = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$);";

/// Parse `records` under `FILE_SCHEMA((schema))`, requiring a clean parse.
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
    assert!(
        model.diagnostics().is_empty(),
        "fixture must parse cleanly: {:?}",
        model.diagnostics()
    );
    model
}

fn ifc2x3(records: &[&str]) -> Model {
    step("IFC2X3", records)
}

fn as_refs(records: &[String]) -> Vec<&str> {
    records.iter().map(String::as_str).collect()
}

/// `IfcPropertySet` #`id` named `name` holding `properties`.
fn pset(id: u64, name: &str, properties: &[u64]) -> String {
    let refs: Vec<String> = properties.iter().map(|p| format!("#{p}")).collect();
    format!(
        "#{id}=IFCPROPERTYSET('{}',$,'{name}',$,({}));",
        guid(id),
        refs.join(",")
    )
}

/// `IfcRelDefinesByProperties` #`id` relating `objects` to definition `set`.
fn defines(id: u64, objects: &[u64], set: &str) -> String {
    let refs: Vec<String> = objects.iter().map(|o| format!("#{o}")).collect();
    format!(
        "#{id}=IFCRELDEFINESBYPROPERTIES('{}',$,$,$,({}),{set});",
        guid(id),
        refs.join(",")
    )
}

/// `IfcRelDefinesByType` #`id` relating `objects` to type object `type_id`.
fn typed_by(id: u64, objects: &[u64], type_id: u64) -> String {
    let refs: Vec<String> = objects.iter().map(|o| format!("#{o}")).collect();
    format!(
        "#{id}=IFCRELDEFINESBYTYPE('{}',$,$,$,({}),#{type_id});",
        guid(id),
        refs.join(",")
    )
}

/// A syntactically valid, distinct 22-character GlobalId per entity id.
fn guid(id: u64) -> String {
    format!("{id:0>22}")
}

fn present(result: Result<ExactResolution, ExactPropertyError>) -> ifc_properties::ExactProperty {
    match result {
        Ok(ExactResolution::Present(property)) => property,
        other => panic!("expected a present property, got {other:?}"),
    }
}

#[test]
fn exact_schema_reports_the_declared_release() {
    assert_eq!(exact_schema(&ifc2x3(&[WALL])), Ok(SchemaVersion::Ifc2x3));
    let ifc4_wall = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";
    assert_eq!(
        exact_schema(&step("IFC4", &[ifc4_wall])),
        Ok(SchemaVersion::Ifc4)
    );
    // IFC4X3 is bundled but not yet verified for exact resolution: refused.
    assert!(matches!(
        exact_schema(&step("IFC4X3_ADD2", &[ifc4_wall])),
        Err(ExactPropertyError::UnsupportedSchema { .. })
    ));
    assert!(matches!(
        exact_schema(&step("IFC2X2_FINAL", &[WALL])),
        Err(ExactPropertyError::UnsupportedSchema { .. })
    ));
}

#[test]
fn an_occurrence_property_resolves_with_ifc2x3_arities() {
    let m = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &pset(11, "Pset_WallCommon", &[10]),
        &defines(12, &[1], "#11"),
    ]);
    let property = present(exact_property(
        &m,
        EntityId(1),
        Some("Pset_WallCommon"),
        "FireRating",
    ));
    assert_eq!(property.source, ExactSource::Occurrence);
    assert_eq!(property.property_set.as_ref(), "Pset_WallCommon");
    assert_eq!(property.set_id, EntityId(11));
    assert_eq!(property.property_id, EntityId(10));
    assert_eq!(property.value_type.as_deref(), Some("IFCLABEL"));
    assert_eq!(property.value, ExactValue::Text("F90".into()));
    // Searching every assigned set finds the same property.
    assert_eq!(
        present(exact_property(&m, EntityId(1), None, "FireRating")).property_id,
        EntityId(10)
    );
}

#[test]
fn absence_is_proven_only_after_complete_traversal() {
    let m = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &pset(11, "Pset_WallCommon", &[10]),
        &defines(12, &[1], "#11"),
    ]);
    assert_eq!(
        exact_property(&m, EntityId(1), Some("Pset_WallCommon"), "IsExternal"),
        Ok(ExactResolution::Absent)
    );
    assert_eq!(
        exact_property(&m, EntityId(1), Some("Pset_Other"), "FireRating"),
        Ok(ExactResolution::Absent)
    );
    // An object with no property relationship at all is also a proven absence.
    assert_eq!(
        exact_property(&ifc2x3(&[WALL]), EntityId(1), None, "FireRating"),
        Ok(ExactResolution::Absent)
    );
}

#[test]
fn occurrence_overrides_the_type_and_the_type_fills_the_rest() {
    let m = ifc2x3(&[
        WALL,
        // IfcWallType, IFC2X3 arity (10).
        "#20=IFCWALLTYPE('00000000000000000000w1',$,'WT',$,$,(#21),$,$,$,.STANDARD.);",
        &pset(21, "Pset_WallCommon", &[22, 23]),
        "#22=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F30'),$);",
        "#23=IFCPROPERTYSINGLEVALUE('AcousticRating',$,IFCLABEL('R45'),$);",
        &typed_by(24, &[1], 20),
        &pset(30, "Pset_WallCommon", &[31]),
        "#31=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &defines(32, &[1], "#30"),
    ]);
    let occurrence = present(exact_property(&m, EntityId(1), None, "FireRating"));
    assert_eq!(occurrence.source, ExactSource::Occurrence);
    assert_eq!(occurrence.value, ExactValue::Text("F90".into()));
    let inherited = present(exact_property(&m, EntityId(1), None, "AcousticRating"));
    assert_eq!(inherited.source, ExactSource::Type(EntityId(20)));
    assert_eq!(inherited.set_id, EntityId(21));
    assert_eq!(inherited.value, ExactValue::Text("R45".into()));
}

#[test]
fn a_door_inherits_from_its_ifc2x3_door_style() {
    let m = ifc2x3(&[
        // IfcDoor, IFC2X3 arity (10); IfcDoorStyle (12), IFC2X3's door type.
        "#1=IFCDOOR('1xS3BCk291UvhgP2dvNsgq',$,'Door',$,$,$,$,$,2.1,0.9);",
        "#20=IFCDOORSTYLE('00000000000000000000d1',$,'DS',$,$,(#21),$,$,\
         .SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.);",
        &pset(21, "Pset_DoorCommon", &[22]),
        "#22=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);",
        &typed_by(23, &[1], 20),
    ]);
    let property = present(exact_property(
        &m,
        EntityId(1),
        Some("Pset_DoorCommon"),
        "IsExternal",
    ));
    assert_eq!(property.source, ExactSource::Type(EntityId(20)));
    assert_eq!(property.value_type.as_deref(), Some("IFCBOOLEAN"));
    assert_eq!(property.value, ExactValue::Bool(true));
}

#[test]
fn typed_values_units_and_unknown_logicals_keep_their_identity() {
    let m = ifc2x3(&[
        WALL,
        "#9=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);",
        "#10=IFCPROPERTYSINGLEVALUE('Count',$,IFCINTEGER(3),$);",
        "#11=IFCPROPERTYSINGLEVALUE('Factor',$,IFCREAL(0.25),$);",
        "#12=IFCPROPERTYSINGLEVALUE('Width',$,IFCLENGTHMEASURE(240.),#9);",
        "#13=IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCLOGICAL(.U.),$);",
        "#14=IFCPROPERTYSINGLEVALUE('Empty',$,$,$);",
        &pset(20, "Pset_Test", &[10, 11, 12, 13, 14]),
        &defines(21, &[1], "#20"),
    ]);
    let get = |name| present(exact_property(&m, EntityId(1), Some("Pset_Test"), name));
    assert_eq!(get("Count").value, ExactValue::Integer(3));
    assert_eq!(get("Count").value_type.as_deref(), Some("IFCINTEGER"));
    assert_eq!(get("Factor").value, ExactValue::Real(0.25));
    let width = get("Width");
    assert_eq!(width.value, ExactValue::Real(240.0));
    assert_eq!(width.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
    assert_eq!(width.unit_id, Some(EntityId(9)));
    assert_eq!(
        get("LoadBearing").value,
        ExactValue::Logical(ExactLogical::Unknown)
    );
    let empty = get("Empty");
    assert_eq!(empty.value, ExactValue::Null);
    assert_eq!(empty.value_type, None);
}

#[test]
fn slot_counts_are_the_declared_release_s_not_ifc4_s() {
    let records = |wall: &str| {
        vec![
            wall.to_string(),
            "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);".into(),
            pset(11, "Pset_WallCommon", &[10]),
            defines(12, &[1], "#11"),
        ]
    };
    let ifc4_wall = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";

    // The same 8-slot wall resolves under IFC2X3 and is malformed under IFC4.
    let eight = records(WALL);
    assert!(exact_property(
        &step("IFC2X3", &as_refs(&eight)),
        EntityId(1),
        None,
        "FireRating"
    )
    .is_ok_and(|r| matches!(r, ExactResolution::Present(_))));
    assert_eq!(
        exact_property(
            &step("IFC4", &as_refs(&eight)),
            EntityId(1),
            None,
            "FireRating"
        ),
        Err(ExactPropertyError::MalformedEntitySlots {
            entity: EntityId(1),
            type_name: "IFCWALL".into(),
            expected: 9,
            actual: 8,
        })
    );
    // And the 9-slot IFC4 wall is malformed under IFC2X3.
    let nine = records(ifc4_wall);
    assert_eq!(
        exact_property(
            &step("IFC2X3", &as_refs(&nine)),
            EntityId(1),
            None,
            "FireRating"
        ),
        Err(ExactPropertyError::MalformedEntitySlots {
            entity: EntityId(1),
            type_name: "IFCWALL".into(),
            expected: 8,
            actual: 9,
        })
    );
}

#[test]
fn ifc4_only_constructs_in_an_ifc2x3_file_fail_closed() {
    // An IfcPropertySetDefinitionSet as the relating definition.
    let set = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &pset(11, "Pset_WallCommon", &[10]),
        &defines(12, &[1], "IFCPROPERTYSETDEFINITIONSET((#11))"),
    ]);
    assert!(matches!(
        exact_property(&set, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::NotInSchema { entity: EntityId(12), ref name, schema: SchemaVersion::Ifc2x3 })
            if name.as_ref() == "IFCPROPERTYSETDEFINITIONSET"
    ));
    // Its untyped list form is just a malformed IFC2X3 reference.
    let bare = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &pset(11, "Pset_WallCommon", &[10]),
        &defines(12, &[1], "(#11)"),
    ]);
    assert!(matches!(
        exact_property(&bare, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::MalformedAggregate {
            entity: EntityId(12),
            attribute: "RelatingPropertyDefinition"
        })
    ));

    // An IfcDoorType (IFC4 arity, 13) as the relating type.
    let door_type = ifc2x3(&[
        "#1=IFCDOOR('1xS3BCk291UvhgP2dvNsgq',$,'Door',$,$,$,$,$,2.1,0.9);",
        "#20=IFCDOORTYPE('00000000000000000000d1',$,'DT',$,$,(#21),$,$,$,.DOOR.,\
         .SINGLE_SWING_LEFT.,.F.,$);",
        &pset(21, "Pset_DoorCommon", &[22]),
        "#22=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);",
        &typed_by(23, &[1], 20),
    ]);
    assert!(matches!(
        exact_property(&door_type, EntityId(1), None, "IsExternal"),
        Err(ExactPropertyError::NotInSchema { entity: EntityId(20), ref name, schema: SchemaVersion::Ifc2x3 })
            if name.as_ref() == "IFCDOORTYPE"
    ));

    // Value types IFC4 added to IfcSimpleValue.
    for value in ["IFCBINARY(\"0FF\")", "IFCPOSITIVEINTEGER(3)"] {
        let property = format!("#10=IFCPROPERTYSINGLEVALUE('P',$,{value},$);");
        let m = ifc2x3(&[
            WALL,
            &property,
            &pset(11, "S", &[10]),
            &defines(12, &[1], "#11"),
        ]);
        assert!(
            matches!(
                exact_property(&m, EntityId(1), Some("S"), "P"),
                Err(ExactPropertyError::NotInSchema {
                    entity: EntityId(10),
                    schema: SchemaVersion::Ifc2x3,
                    ..
                })
            ),
            "{value}"
        );
    }

    // A unit entity IFC4 has and IFC2X3 does not. `IfcSIUnit` exists in both,
    // so use a record name no IFC2X3 table declares: the unit path must name
    // the foreign construct, not merely report a slot mismatch.
    let foreign_unit = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('P',$,IFCLENGTHMEASURE(1.),#13);",
        &pset(11, "S", &[10]),
        &defines(12, &[1], "#11"),
        "#13=IFCCOORDINATEREFERENCESYSTEM('EPSG:25832',$,$,$);",
    ]);
    assert!(matches!(
        exact_property(&foreign_unit, EntityId(1), Some("S"), "P"),
        Err(ExactPropertyError::NotInSchema {
            entity: EntityId(13),
            schema: SchemaVersion::Ifc2x3,
            ..
        })
    ));
    // IFC4 adds no concrete property-set definition over IFC2X3 (its two
    // additions, `IfcPreDefinedPropertySet` and `IfcQuantitySet`, are
    // abstract), so a foreign set can only be a name no release declares.
    // It must be named as foreign before any arity check.
    let foreign_set = ifc2x3(&[
        WALL,
        "#11=IFCVENDORPROPERTYSET('00000000000000000000s1',$,'S',$,(#10));",
        "#10=IFCPROPERTYSINGLEVALUE('P',$,IFCLABEL('x'),$);",
        &defines(12, &[1], "#11"),
    ]);
    assert!(matches!(
        exact_property(&foreign_set, EntityId(1), None, "P"),
        Err(ExactPropertyError::NotInSchema {
            entity: EntityId(11),
            schema: SchemaVersion::Ifc2x3,
            ..
        })
    ));
}

#[test]
fn the_same_ifc4_constructs_resolve_under_an_ifc4_header() {
    // Control for the negative test: the declared release, not the
    // construct, decides.
    let m = step(
        "IFC4",
        &[
            "#1=IFCDOOR('1xS3BCk291UvhgP2dvNsgq',$,'Door',$,$,$,$,$,2.1,0.9,$,$,$);",
            "#20=IFCDOORTYPE('00000000000000000000d1',$,'DT',$,$,(#21),$,$,$,.DOOR.,\
             .SINGLE_SWING_LEFT.,.F.,$);",
            &pset(21, "Pset_DoorCommon", &[22]),
            "#22=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);",
            &typed_by(23, &[1], 20),
            "#30=IFCPROPERTYSINGLEVALUE('Count',$,IFCPOSITIVEINTEGER(3),$);",
            &pset(31, "S", &[30]),
            &defines(32, &[1], "IFCPROPERTYSETDEFINITIONSET((#31))"),
        ],
    );
    assert_eq!(
        present(exact_property(&m, EntityId(1), None, "IsExternal")).source,
        ExactSource::Type(EntityId(20))
    );
    assert_eq!(
        present(exact_property(&m, EntityId(1), Some("S"), "Count")).value,
        ExactValue::Integer(3)
    );
}

#[test]
fn a_property_override_relating_the_object_is_refused_not_ignored() {
    // IFC2X3 `IfcRelOverridesProperties` (7 slots) is a subtype of
    // `IfcRelDefinesByProperties` whose OverridingProperties replace values.
    // Answering without it would be wrong, so the object is refused.
    let overrides = |related: u64| {
        ifc2x3(&[
            WALL,
            "#2=IFCWALL('1xS3BCk291UvhgP2dvNsgr',$,'Other',$,$,$,$,$);",
            "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
            &pset(11, "Pset_WallCommon", &[10]),
            &defines(12, &[1, 2], "#11"),
            "#13=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F120'),$);",
            &format!(
                "#14=IFCRELOVERRIDESPROPERTIES('{}',$,$,$,(#{related}),#11,(#13));",
                guid(14)
            ),
        ])
    };
    assert!(matches!(
        exact_property(&overrides(1), EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::UnsupportedRelationship { relationship: EntityId(14), ref type_name })
            if type_name.as_ref() == "IFCRELOVERRIDESPROPERTIES"
    ));
    // An override on a different object does not touch this answer.
    assert_eq!(
        present(exact_property(
            &overrides(2),
            EntityId(1),
            None,
            "FireRating"
        ))
        .value,
        ExactValue::Text("F90".into())
    );
}

#[test]
fn ifc2x3_domains_fail_closed() {
    // A type object is not an occurrence in either release.
    let type_as_occurrence = ifc2x3(&[
        WALL,
        "#20=IFCWALLTYPE('00000000000000000000w1',$,'WT',$,$,$,$,$,$,.STANDARD.);",
        "#10=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F90'),$);",
        &pset(11, "Pset_WallCommon", &[10]),
        &defines(12, &[20], "#11"),
    ]);
    assert_eq!(
        exact_property(&type_as_occurrence, EntityId(1), None, "FireRating"),
        Err(ExactPropertyError::InvalidOccurrenceTarget {
            relationship: EntityId(12),
            object: EntityId(20),
        })
    );
    assert!(matches!(
        exact_property(&type_as_occurrence, EntityId(20), None, "FireRating"),
        Err(ExactPropertyError::InvalidQueryObject {
            object: EntityId(20),
            ..
        })
    ));
    // IFC2X3's occurrence domain is `IfcObject`, not IFC4's
    // `IfcObjectDefinition`. The one IFC2X3 entity in between is the abstract
    // `IfcObjectDefinition` itself; an instance of it is not an occurrence.
    let bare_definition = ifc2x3(&[
        WALL,
        "#30=IFCOBJECTDEFINITION('00000000000000000000o1',$,'D',$);",
    ]);
    assert!(matches!(
        exact_property(&bare_definition, EntityId(30), None, "FireRating"),
        Err(ExactPropertyError::InvalidQueryObject {
            object: EntityId(30),
            ..
        })
    ));
    // A type's `HasPropertySets` naming a set no IFC2X3 table declares is
    // foreign, not a slot mismatch: the type path is checked like the
    // occurrence path.
    let foreign_type_set = ifc2x3(&[
        WALL,
        "#20=IFCWALLTYPE('00000000000000000000w1',$,'WT',$,$,(#21),$,$,$,.STANDARD.);",
        "#21=IFCVENDORPROPERTYSET('00000000000000000000s1',$,'S',$,(#22));",
        "#22=IFCPROPERTYSINGLEVALUE('P',$,IFCLABEL('x'),$);",
        &typed_by(23, &[1], 20),
    ]);
    assert!(matches!(
        exact_property(&foreign_type_set, EntityId(1), None, "P"),
        Err(ExactPropertyError::NotInSchema {
            entity: EntityId(21),
            schema: SchemaVersion::Ifc2x3,
            ..
        })
    ));
    // A unit reference that is not an IfcUnit.
    let bad_unit = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('Width',$,IFCLENGTHMEASURE(240.),#1);",
        &pset(11, "S", &[10]),
        &defines(12, &[1], "#11"),
    ]);
    assert_eq!(
        exact_property(&bad_unit, EntityId(1), Some("S"), "Width"),
        Err(ExactPropertyError::UnsupportedUnit {
            property: EntityId(10)
        })
    );
    // A property record with IFC2X3's arity but a value outside IfcValue.
    let bad_value = ifc2x3(&[
        WALL,
        "#10=IFCPROPERTYSINGLEVALUE('P',$,IFCTEXT(7),$);",
        &pset(11, "S", &[10]),
        &defines(12, &[1], "#11"),
    ]);
    assert_eq!(
        exact_property(&bad_value, EntityId(1), Some("S"), "P"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
}

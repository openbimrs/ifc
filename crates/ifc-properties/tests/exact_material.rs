//! Exact material property sets (#218).
//!
//! The buildingSMART IDS corpus checks a material's properties through
//! `IfcMaterialProperties` (IFC4, IFC4X3) and `IfcExtendedMaterialProperties`
//! (IFC2X3), and requires an absent property to fail. Both need an exact
//! reader: a present value with its provenance, and an absence that is
//! proven rather than a lookup failure.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_material_properties_where, exact_material_property, exact_material_property_sets_where,
    exact_property, ExactPropertyError, ExactResolution, ExactSource, ExactValue,
};
use ifc_step::StepCodec;

const MATERIAL: EntityId = EntityId(1);

fn parse(schema: &str, records: &[&str]) -> Model {
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

/// An IFC4 or IFC4X3 material (#1) with `extra` records.
fn ifc4(schema: &str, extra: &[&str]) -> Model {
    let mut records = vec!["#1=IFCMATERIAL('Concrete',$,$);"];
    records.extend(extra);
    parse(schema, &records)
}

/// An IFC2X3 material (#1) with `extra` records.
fn ifc2x3(extra: &[&str]) -> Model {
    let mut records = vec!["#1=IFCMATERIAL('Concrete');"];
    records.extend(extra);
    parse("IFC2X3", &records)
}

fn present(result: Result<ExactResolution, ExactPropertyError>) -> ifc_properties::ExactProperty {
    match result {
        Ok(ExactResolution::Present(property)) => property,
        other => panic!("expected a present property, got {other:?}"),
    }
}

/// `pass-material_properties_are_supported_under_ifc4_via_ifcmaterialproperties`
/// and its IFC4X3 counterpart, with the matching absent-property cases.
#[test]
fn material_properties_resolve_in_ifc4_and_ifc4x3() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = ifc4(
            schema,
            &[
                "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
                "#3=IFCMATERIALPROPERTIES('Foo_Bar',$,(#2),#1);",
            ],
        );
        for set in [Some("Foo_Bar"), None] {
            let found = present(exact_material_property(&model, MATERIAL, set, "Foo"));
            assert_eq!(found.source, ExactSource::Material(MATERIAL), "{schema}");
            assert_eq!(&*found.property_set, "Foo_Bar");
            assert_eq!(found.set_id, EntityId(3));
            assert_eq!(found.property_id, EntityId(2));
            assert_eq!(found.value_type.as_deref(), Some("IFCLABEL"));
            assert_eq!(found.value, ExactValue::Text("Bar".into()));
        }
        assert_eq!(
            exact_material_property(&model, MATERIAL, Some("Foo_Bar"), "Missing"),
            Ok(ExactResolution::Absent)
        );
        assert_eq!(
            exact_material_property(&model, MATERIAL, Some("Other"), "Foo"),
            Ok(ExactResolution::Absent)
        );
        // A material with no property sets at all is a proven absence.
        let bare = ifc4(schema, &[]);
        assert_eq!(
            exact_material_property(&bare, MATERIAL, None, "Foo"),
            Ok(ExactResolution::Absent)
        );
        assert_eq!(
            exact_material_property_sets_where(&bare, MATERIAL, |_| true),
            Ok(Vec::new())
        );
    }
}

/// `pass-material_properties_are_supported_under_ifc2x3_via_extendedmaterialproperties`
/// and `fail-material_properties_that_are_absent_fail_under_ifc2x3`.
#[test]
fn extended_material_properties_resolve_in_ifc2x3() {
    let model = ifc2x3(&[
        "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
        "#3=IFCEXTENDEDMATERIALPROPERTIES(#1,(#2),$,'Foo_Bar');",
    ]);
    let found = present(exact_material_property(
        &model,
        MATERIAL,
        Some("Foo_Bar"),
        "Foo",
    ));
    assert_eq!(found.source, ExactSource::Material(MATERIAL));
    assert_eq!(found.set_id, EntityId(3));
    assert_eq!(found.value, ExactValue::Text("Bar".into()));
    assert_eq!(
        exact_material_property(&model, MATERIAL, Some("Foo_Bar"), "Missing"),
        Ok(ExactResolution::Absent)
    );
    assert_eq!(
        exact_material_property(&ifc2x3(&[]), MATERIAL, Some("Foo_Bar"), "Foo"),
        Ok(ExactResolution::Absent)
    );
}

/// An IFC2X3 typed subtype holds its values in its own attributes, keyed by
/// its entity name, as a predefined object set is.
#[test]
fn typed_ifc2x3_material_properties_resolve_as_attributes() {
    let model = ifc2x3(&["#3=IFCGENERALMATERIALPROPERTIES(#1,$,0.2,2400.);"]);
    let density = present(exact_material_property(
        &model,
        MATERIAL,
        Some("IfcGeneralMaterialProperties"),
        "MassDensity",
    ));
    assert_eq!(density.property_id, EntityId(3));
    assert_eq!(density.value_type.as_deref(), Some("IFCMASSDENSITYMEASURE"));
    assert_eq!(density.value, ExactValue::Real(2400.0));
    let weight = present(exact_material_property(
        &model,
        MATERIAL,
        None,
        "MolecularWeight",
    ));
    assert_eq!(weight.value, ExactValue::Null);
    // `Material` is the link to the material, not a property.
    assert_eq!(
        exact_material_property(&model, MATERIAL, None, "Material"),
        Ok(ExactResolution::Absent)
    );
}

#[test]
fn enumeration_lists_every_set_and_property() {
    let model = ifc4(
        "IFC4",
        &[
            "#2=IFCPROPERTYSINGLEVALUE('A',$,IFCINTEGER(1),$);",
            "#3=IFCMATERIALPROPERTIES('Pset_One',$,(#2),#1);",
            "#4=IFCPROPERTYSINGLEVALUE('B',$,IFCREAL(2.5),$);",
            // An unnamed set is keyed by its entity name.
            "#5=IFCMATERIALPROPERTIES($,$,(#4),#1);",
            // A set of another material is not listed.
            "#6=IFCMATERIAL('Steel',$,$);",
            "#7=IFCPROPERTYSINGLEVALUE('C',$,IFCINTEGER(3),$);",
            "#8=IFCMATERIALPROPERTIES('Pset_One',$,(#7),#6);",
            // An empty set exists and holds nothing.
            "#9=IFCMATERIALPROPERTIES('Pset_Empty',$,(),#1);",
        ],
    );
    let sets = exact_material_property_sets_where(&model, MATERIAL, |_| true).expect("readable");
    let listed: Vec<(&str, EntityId, usize)> = sets
        .iter()
        .map(|set| (&*set.name, set.set_id, set.members))
        .collect();
    assert_eq!(
        listed,
        [
            ("Pset_One", EntityId(3), 1),
            ("IfcMaterialProperties", EntityId(5), 1),
            ("Pset_Empty", EntityId(9), 0),
        ]
    );
    assert!(sets
        .iter()
        .all(|set| set.source == ExactSource::Material(MATERIAL)));
    let properties =
        exact_material_properties_where(&model, MATERIAL, |set| set != "Pset_Empty", |_| true)
            .expect("readable");
    let names: Vec<&str> = properties.iter().map(|entry| &*entry.name).collect();
    assert_eq!(names, ["A", "B"]);
    assert_eq!(
        present(exact_material_property(
            &model,
            MATERIAL,
            Some("IfcMaterialProperties"),
            "B"
        ))
        .value,
        ExactValue::Real(2.5)
    );
}

#[test]
fn every_material_definition_is_a_query_target_in_ifc4() {
    let model = parse(
        "IFC4",
        &[
            "#1=IFCMATERIAL('Concrete',$,$);",
            "#2=IFCMATERIALLAYER(#1,0.2,$,'Core',$,$,$);",
            "#3=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
            "#4=IFCMATERIALPROPERTIES('Foo_Bar',$,(#3),#2);",
        ],
    );
    let found = present(exact_material_property(&model, EntityId(2), None, "Foo"));
    assert_eq!(found.source, ExactSource::Material(EntityId(2)));
    // The layer's set is the layer's own, not its material's.
    assert_eq!(
        exact_material_property(&model, MATERIAL, None, "Foo"),
        Ok(ExactResolution::Absent)
    );
}

#[test]
fn malformed_queries_and_sets_are_refused() {
    // Not a material.
    let wall = parse(
        "IFC4",
        &["#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);"],
    );
    assert!(matches!(
        exact_material_property(&wall, MATERIAL, None, "Foo"),
        Err(ExactPropertyError::InvalidQueryObject { object, .. }) if object == MATERIAL
    ));
    // IFC2X3 relates material properties to an `IfcMaterial` only.
    let layer = ifc2x3(&["#2=IFCMATERIALLAYER(#1,0.2,$);"]);
    assert!(matches!(
        exact_material_property(&layer, EntityId(2), None, "Foo"),
        Err(ExactPropertyError::InvalidQueryObject { object, .. }) if object == EntityId(2)
    ));
    // A set of any material with a dangling `Material` could hide a set.
    let dangling = ifc4(
        "IFC4",
        &[
            "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
            "#3=IFCMATERIALPROPERTIES('Foo_Bar',$,(#2),#99);",
        ],
    );
    assert_eq!(
        exact_material_property(&dangling, MATERIAL, None, "Foo"),
        Err(ExactPropertyError::MissingReference {
            from: EntityId(3),
            to: EntityId(99),
        })
    );
    // Two sets of one name on one material are ambiguous.
    let twice = ifc4(
        "IFC4",
        &[
            "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
            "#3=IFCMATERIALPROPERTIES('Foo_Bar',$,(#2),#1);",
            "#4=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Baz'),$);",
            "#5=IFCMATERIALPROPERTIES('Foo_Bar',$,(#4),#1);",
        ],
    );
    assert_eq!(
        exact_material_property(&twice, MATERIAL, Some("Foo_Bar"), "Foo"),
        Err(ExactPropertyError::DuplicateMatchingSets {
            source: ExactSource::Material(MATERIAL),
            first: EntityId(3),
            second: EntityId(5),
        })
    );
    // IFC2X3 requires the extended set's `Name`.
    let unnamed = ifc2x3(&[
        "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
        "#3=IFCEXTENDEDMATERIALPROPERTIES(#1,(#2),$,$);",
    ]);
    assert_eq!(
        exact_material_property(&unnamed, MATERIAL, None, "Foo"),
        Err(ExactPropertyError::MalformedName {
            entity: EntityId(3),
            attribute: "Name",
        })
    );
}

/// Material property sets never reach an object's resolution: an
/// `IfcMaterialProperties` in a type's `HasPropertySets` is still refused.
#[test]
fn object_resolution_still_refuses_material_sets() {
    let model = parse(
        "IFC4",
        &[
            "#1=IFCMATERIAL('Concrete',$,$);",
            "#2=IFCPROPERTYSINGLEVALUE('Foo',$,IFCLABEL('Bar'),$);",
            "#3=IFCMATERIALPROPERTIES('Foo_Bar',$,(#2),#1);",
            "#4=IFCWALLTYPE('0000000000000000000004',$,'WT',$,$,(#3),$,$,$,.STANDARD.);",
        ],
    );
    assert!(matches!(
        exact_property(&model, EntityId(4), None, "Foo"),
        Err(ExactPropertyError::UnsupportedDefinition { entity, .. }) if entity == EntityId(3)
    ));
}

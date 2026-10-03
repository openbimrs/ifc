//! Native tests of writing property sets (#123, part 2), the operation every
//! host binds: values written are read back identically through the read
//! side, in memory, through STEP and through ifcXML; each refusal has its
//! code; and a refused batch leaves the model byte-for-byte unchanged.
//!
//! The gate runs this file with the catalog (default features), with the
//! writer but no catalog (`ifc4,properties-write`), with a runtime catalog
//! it never loads (`ifc4,property-catalog-runtime`; `tests/catalog.rs`
//! loads one), and with neither (`ifc4`, and `ifc4,georef`, which reads
//! property sets but cannot write).

use openbim_ifc_binding_core::property_edit::PropertyEdit;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::IfcModel;

#[cfg_attr(not(feature = "properties-write"), allow(dead_code))]
fn fixture() -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

fn typed(type_name: &str, value: Tagged) -> Tagged {
    Tagged::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

fn label(text: &str) -> Tagged {
    typed("IFCLABEL", Tagged::Text(text.into()))
}

/// The code of a refused batch, after checking the model did not change.
#[cfg_attr(not(feature = "properties-write"), allow(dead_code))]
fn refused(model: &mut IfcModel, edits: Vec<PropertyEdit>) -> &'static str {
    let before = model.write().expect("writes");
    let error = model.set_properties(edits).expect_err("refused");
    assert_eq!(
        model.write().expect("writes"),
        before,
        "a refused batch changed the model: {error}"
    );
    error.code()
}

#[cfg(feature = "properties-write")]
mod with_properties {
    use super::*;
    use openbim_ifc_binding_core::property_edit::EditAction;

    /// `object`'s `set.name` as the read side reports it: (source, value).
    fn read(model: &IfcModel, object: u64, set: &str, name: &str) -> Option<(String, Tagged)> {
        model
            .property_sets(object)
            .expect("reads")
            .into_iter()
            .filter(|s| s.name == set)
            .flat_map(|s| {
                let source = s.source.clone();
                s.properties.into_iter().map(move |p| (source.clone(), p))
            })
            .find(|(_, p)| p.name == name)
            .map(|(source, p)| (source, p.value))
    }

    /// Custom sets, which no catalog describes: they run in every build
    /// with property sets. Wall B (#31) holds `Pset_Families`; this writes
    /// a set of its own beside it.
    fn custom_edits() -> Vec<PropertyEdit> {
        let mut quantity = PropertyEdit::set(
            31,
            "Custom_Quantities",
            "Depth",
            typed("IFCLENGTHMEASURE", Tagged::Real(0.25)),
        );
        if let EditAction::Set { set_type, .. } = &mut quantity.action {
            *set_type = Some("IfcElementQuantity".into());
        }
        vec![
            PropertyEdit::set(31, "Custom", "Note", label("checked")),
            PropertyEdit::set(
                31,
                "Custom",
                "Load",
                typed("IFCFORCEMEASURE", Tagged::Real(1.5)),
            ),
            PropertyEdit::set(31, "Custom", "Unset", Tagged::Null),
            quantity,
        ]
    }

    fn written(model: &IfcModel) -> Vec<Option<(String, Tagged)>> {
        [
            ("Custom", "Note"),
            ("Custom", "Load"),
            ("Custom", "Unset"),
            ("Custom_Quantities", "Depth"),
        ]
        .into_iter()
        .map(|(set, name)| read(model, 31, set, name))
        .collect()
    }

    #[test]
    fn written_values_read_back_identically_in_memory_and_through_step() {
        let mut model = fixture();
        let edits = custom_edits();
        let expected: Vec<Option<(String, Tagged)>> = edits
            .iter()
            .map(|edit| match &edit.action {
                EditAction::Set { value, .. } => Some(("occurrence".to_owned(), value.clone())),
                EditAction::Remove => None,
            })
            .collect();
        let result = model.set_properties(edits).expect("applies");
        assert_eq!(result.properties.len(), 4);
        assert!(result.properties.iter().all(Option::is_some));
        assert_eq!(written(&model), expected, "in memory");

        let reread = IfcModel::parse(&model.write().unwrap()).expect("re-reads");
        assert_eq!(written(&reread), expected, "through STEP");
    }

    #[cfg(feature = "ifcxml")]
    #[test]
    fn written_values_read_back_identically_through_ifcxml() {
        let mut model = fixture();
        model.set_properties(custom_edits()).expect("applies");
        let expected = written(&model);
        let xml = model.write_ifcxml(None).expect("writes ifcXML");
        let reread = IfcModel::parse_ifcxml(&xml, None).expect("re-reads ifcXML");
        assert_eq!(written(&reread), expected);
        assert!(expected.iter().all(Option::is_some));
    }

    #[test]
    fn single_edits_are_thin_wrappers() {
        let mut model = fixture();
        let id = model
            .set_property(31, "Custom", "Note", label("one"), None)
            .expect("sets");
        assert_eq!(model.type_of(id).unwrap(), "IFCPROPERTYSINGLEVALUE");
        model
            .remove_property(31, "Custom", "Note")
            .expect("removes");
        assert!(read(&model, 31, "Custom", "Note").is_none());
    }

    #[test]
    fn every_refusal_has_its_code_and_changes_nothing() {
        let mut model = fixture();
        let cases: Vec<(&str, Vec<PropertyEdit>)> = vec![
            (
                "missing-entity",
                vec![PropertyEdit::set(999, "Custom", "A", label("x"))],
            ),
            // #19 is a cartesian point: it carries no property sets.
            (
                "wrong-entity-type",
                vec![PropertyEdit::set(19, "Custom", "A", label("x"))],
            ),
            (
                "invalid-value",
                vec![PropertyEdit::set(31, "Custom", "A", Tagged::Real(1.0))],
            ),
            (
                "invalid-value",
                vec![PropertyEdit::set(
                    31,
                    "Custom",
                    "A",
                    typed("IFCGLOBALLYUNIQUEID", Tagged::Text("x".into())),
                )],
            ),
            (
                "missing-property",
                vec![PropertyEdit::remove(31, "Custom", "A")],
            ),
            (
                "unsupported",
                vec![PropertyEdit::set(
                    31,
                    "Pset_Families",
                    "Range",
                    typed("IFCLENGTHMEASURE", Tagged::Real(1.0)),
                )],
            ),
            // A batch is refused as a whole: the first edit is valid.
            (
                "missing-property",
                vec![
                    PropertyEdit::set(31, "Custom", "A", label("x")),
                    PropertyEdit::remove(31, "Custom", "B"),
                ],
            ),
        ];
        for (code, edits) in cases {
            // Pset_Families is a catalog-prefixed name; without the catalog
            // its refusal is `feature-disabled`, and with a runtime catalog
            // this file never loads, `catalog-not-loaded`.
            let expected = if cfg!(feature = "property-catalog")
                || !edits.iter().any(|e| e.set.starts_with("Pset_"))
            {
                code
            } else if cfg!(feature = "property-catalog-runtime") {
                "catalog-not-loaded"
            } else {
                "feature-disabled"
            };
            assert_eq!(refused(&mut model, edits.clone()), expected, "{edits:?}");
        }
    }

    #[test]
    fn a_release_the_read_side_does_not_read_is_refused() {
        let text = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .replacen("FILE_SCHEMA(('IFC4'))", "FILE_SCHEMA(('IFC4X1'))", 1);
        let mut model = IfcModel::parse(text.as_bytes()).expect("parses");
        assert_eq!(
            refused(
                &mut model,
                vec![PropertyEdit::set(31, "Custom", "A", label("x"))]
            ),
            "unsupported-schema"
        );
    }

    #[test]
    fn the_c_tape_form_reads_both_actions() {
        let set = Tagged::List(vec![
            Tagged::Enum("SET".into()),
            Tagged::Ref(31),
            Tagged::Text("Custom".into()),
            Tagged::Text("Note".into()),
            label("x"),
            Tagged::Null,
        ]);
        assert_eq!(
            PropertyEdit::from_tagged(&set).unwrap(),
            PropertyEdit::set(31, "Custom", "Note", label("x"))
        );
        let remove = Tagged::List(vec![
            Tagged::Enum("REMOVE".into()),
            Tagged::Integer(31),
            Tagged::Text("Custom".into()),
            Tagged::Text("Note".into()),
        ]);
        assert_eq!(
            PropertyEdit::from_tagged(&remove).unwrap(),
            PropertyEdit::remove(31, "Custom", "Note")
        );
        for bad in [
            Tagged::Text("x".into()),
            Tagged::List(vec![Tagged::Enum("REMOVE".into()), Tagged::Ref(1)]),
            Tagged::List(vec![
                Tagged::Enum("MOVE".into()),
                Tagged::Ref(1),
                Tagged::Text("S".into()),
                Tagged::Text("P".into()),
            ]),
        ] {
            assert_eq!(
                PropertyEdit::from_tagged(&bad).unwrap_err().code(),
                "invalid-value"
            );
        }
    }

    #[cfg(feature = "property-catalog")]
    #[test]
    fn catalog_sets_are_checked_and_types_are_never_changed_through_an_occurrence() {
        let mut model = fixture();
        let type_value = model.attributes(35).unwrap();
        // Wall B inherits FireRating from type #29: the write overrides it.
        model
            .set_property(31, "Pset_WallCommon", "FireRating", label("F60"), None)
            .expect("overrides");
        assert_eq!(
            model.attributes(35).unwrap(),
            type_value,
            "the type's value changed"
        );
        assert_eq!(
            read(&model, 31, "Pset_WallCommon", "FireRating"),
            Some(("occurrence".to_owned(), label("F60")))
        );
        // Addressing the type changes the type's own set.
        model
            .set_property(29, "Pset_WallCommon", "FireRating", label("F90"), None)
            .expect("edits the type");
        assert_eq!(
            read(&model, 31, "Pset_WallCommon", "FireRating"),
            Some(("occurrence".to_owned(), label("F60"))),
            "the override still wins"
        );
        assert_eq!(
            read(&model, 30, "Pset_WallCommon", "FireRating"),
            Some(("type".to_owned(), label("F90")))
        );
        // A quantity is written back in the read side's typed form.
        let width = typed("IFCLENGTHMEASURE", Tagged::Real(250.0));
        model
            .set_property(30, "Qto_WallBaseQuantities", "Width", width.clone(), None)
            .expect("sets the quantity");
        assert_eq!(
            read(&model, 30, "Qto_WallBaseQuantities", "Width")
                .unwrap()
                .1,
            width
        );

        assert_eq!(
            refused(
                &mut model,
                vec![PropertyEdit::set(
                    30,
                    "Pset_WallCommon",
                    "FireRating",
                    typed("IFCREAL", Tagged::Real(1.0))
                )]
            ),
            "template-violation"
        );
        assert_eq!(
            refused(
                &mut model,
                vec![PropertyEdit::remove(31, "Pset_WallCommon", "IsExternal")]
            ),
            "missing-property",
            "inherited only"
        );
        assert_eq!(
            refused(
                &mut model,
                vec![PropertyEdit::set(
                    31,
                    "Pset_Families",
                    "Colour",
                    Tagged::List(vec![label("blue")])
                )]
            ),
            "template-violation",
            "not in IfcPropertyEnumeration #40"
        );
    }

    #[cfg(not(any(feature = "property-catalog", feature = "property-catalog-runtime")))]
    #[test]
    fn a_catalog_set_needs_the_catalog() {
        let mut model = fixture();
        assert_eq!(
            refused(
                &mut model,
                vec![PropertyEdit::set(
                    30,
                    "Pset_WallCommon",
                    "FireRating",
                    label("F60")
                )]
            ),
            "feature-disabled"
        );
    }
}

#[cfg(not(feature = "properties-write"))]
#[test]
fn writing_without_the_feature_refuses() {
    let mut model = IfcModel::empty();
    let error = model
        .set_properties(vec![PropertyEdit::set(1, "Custom", "A", label("x"))])
        .unwrap_err();
    assert_eq!(
        error,
        openbim_ifc_binding_core::BindingError::FeatureDisabled("properties-write")
    );
}

/// A release the build leaves out is refused typed, not read through
/// another release's table (#306).
#[cfg(all(feature = "properties-write", not(feature = "ifc2x3")))]
#[test]
fn a_release_left_out_of_the_build_is_refused() {
    let text = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC2X3'));ENDSEC;
DATA;
#1=IFCOWNERHISTORY($,$,$,.ADDED.,$,$,$,0);
#2=IFCBUILDINGELEMENTPROXY('2YvctVUKr0kugbFTf53O9L',#1,'P',$,$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;
";
    let mut model = IfcModel::parse(text.as_bytes()).expect("parses");
    assert_eq!(
        refused(
            &mut model,
            vec![PropertyEdit::set(2, "Custom", "A", label("x"))]
        ),
        "unsupported-schema"
    );
}

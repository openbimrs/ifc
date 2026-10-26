//! Selectors and values by path every host binds (#464), over the IFC2X3,
//! IFC4 and IFC4X3 selector fixtures. The facade's own suite covers the
//! grammar and every facet; this one the binding's answers, its error
//! codes and spans, and its plain-value writes. The hosts' suites only
//! check their conversion of these answers.

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{
    canonical_selector, BindingError, IfcModel, Plain, Selected, SELECTOR_GRAMMAR_VERSION,
};

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// The fixtures this build bundles a release for.
fn fixtures() -> Vec<IfcModel> {
    let mut out = Vec::new();
    if cfg!(feature = "ifc2x3") {
        out.push(fixture("selector_ifc2x3.ifc"));
    }
    if cfg!(feature = "ifc4") {
        out.push(fixture("selector_ifc4.ifc"));
    }
    if cfg!(feature = "ifc4x3") {
        out.push(fixture("selector_ifc4x3.ifc"));
    }
    assert!(!out.is_empty(), "every build bundles a release");
    out
}

fn text(value: &str) -> Selected {
    Selected::Value(Tagged::Text(value.to_owned()))
}

#[test]
fn selectors_select_in_ascending_id_order() {
    assert_eq!(SELECTOR_GRAMMAR_VERSION, "1.0");
    for model in fixtures() {
        assert_eq!(model.select("IfcWall"), Ok(vec![17, 18, 19]));
        assert_eq!(
            model.select("IfcWall, type=WT-200 + IfcDoor"),
            Ok(vec![17, 18, 28])
        );
        assert_eq!(model.select("material=Oak"), Ok(vec![27, 28]));
        assert_eq!(model.select("location=\"Level 1\""), Ok(vec![19, 38, 40]));
        assert_eq!(model.select_from("IfcWall", &[17, 28]), Ok(vec![17]));
        assert_eq!(
            model.select_from("IfcWall", &[999]),
            Err(BindingError::MissingEntity(999))
        );
        let properties = model.select("Pset_WallCommon.IsExternal=TRUE");
        if cfg!(feature = "properties") {
            assert_eq!(properties, Ok(vec![17]));
        } else {
            assert_eq!(properties, Err(BindingError::FeatureDisabled("properties")));
        }
    }
}

#[test]
fn a_query_that_does_not_parse_names_its_span() {
    for model in fixtures() {
        let error = model.select("IfcWall, colour=red").unwrap_err();
        assert_eq!(error.code(), "invalid-selector");
        assert_eq!(error.span(), Some((9, 15)));
        let error = model.select("IfcWall + IfcWal").unwrap_err();
        assert_eq!(
            (error.code(), error.span()),
            ("invalid-selector", Some((10, 16)))
        );
        let error = model.get_value(17, "type.").unwrap_err();
        assert_eq!(
            (error.code(), error.span()),
            ("invalid-selector", Some((5, 5)))
        );
        assert_eq!(BindingError::MissingEntity(1).span(), None);
    }
    assert_eq!(
        canonical_selector(" IfcWall ,container = \"Level 1\" + ").as_deref(),
        Ok("IfcWall, location=\"Level 1\"")
    );
    assert_eq!(
        canonical_selector("").unwrap_err().code(),
        "invalid-selector"
    );
}

#[test]
fn values_are_read_by_path() {
    for model in fixtures() {
        assert_eq!(model.get_value(17, "type.Name"), Ok(text("WT-200")));
        assert_eq!(
            model.get_value(17, "type"),
            Ok(Selected::Value(Tagged::Ref(16)))
        );
        assert_eq!(
            model.get_value(19, "type.Name"),
            Ok(Selected::Value(Tagged::Null))
        );
        assert_eq!(
            model.get_value(16, "occurrences.Name"),
            Ok(Selected::List(vec![text("Wall A"), text("Wall B")]))
        );
        assert_eq!(
            model.get_value(999, "Name"),
            Err(BindingError::MissingEntity(999))
        );
        if cfg!(feature = "properties") {
            assert_eq!(
                model.get_value(17, "Pset_WallCommon.FireRating"),
                Ok(text("F90"))
            );
            let set = model.get_value(18, "Pset_WallCommon").unwrap();
            let Selected::Properties(members) = &set else {
                panic!("{set:?}");
            };
            assert_eq!(
                members[0],
                (
                    "IsExternal".to_owned(),
                    Selected::Value(Tagged::Bool(false))
                )
            );
            // The C tape form: a list of [name, value] pairs.
            let Tagged::List(pairs) = set.to_tagged() else {
                panic!("a property set crosses as a list");
            };
            assert_eq!(
                pairs[0],
                Tagged::List(vec![Tagged::Text("IsExternal".into()), Tagged::Bool(false)])
            );
        } else {
            let refused = model.get_value(17, "Pset_WallCommon.FireRating");
            assert_eq!(refused, Err(BindingError::FeatureDisabled("properties")));
        }
    }
}

#[test]
fn plain_values_are_written_by_path() {
    for mut model in fixtures() {
        model
            .set_value(17, "type.Name", Plain::Text("WT-300".into()))
            .unwrap();
        assert_eq!(model.get_value(18, "type.Name"), Ok(text("WT-300")));
        let refused = model.set_value(17, "Name", Plain::Integer(3)).unwrap_err();
        assert_eq!(refused.code(), "type-mismatch");
        let refused = model.set_value(17, "count", Plain::Integer(3)).unwrap_err();
        assert_eq!(refused.code(), "invalid-value");
        let refused = model.set_value(17, "Flavour", Plain::Null).unwrap_err();
        assert_eq!(refused.code(), "unknown-attribute");
        let written = model.set_value(19, "Custom_Notes.Note", Plain::Text("x".into()));
        if cfg!(feature = "properties-write") {
            // Nothing states the property, so a plain string has no type.
            assert_eq!(written.unwrap_err().code(), "type-mismatch");
            let typed = Plain::Exact(Tagged::Typed {
                type_name: "IFCLABEL".into(),
                value: Box::new(Tagged::Text("checked".into())),
            });
            model.set_value(19, "Custom_Notes.Note", typed).unwrap();
            model
                .set_value(19, "Custom_Notes.Note", Plain::Text("rechecked".into()))
                .unwrap();
            assert_eq!(
                model.get_value(19, "Custom_Notes.Note"),
                Ok(text("rechecked"))
            );
        } else {
            assert_eq!(
                written,
                Err(BindingError::FeatureDisabled("properties-write"))
            );
        }
    }
}

#[cfg(feature = "ifc4")]
#[test]
fn plain_strings_become_enumeration_items() {
    let mut model = fixture("selector_ifc4.ifc");
    model
        .set_value(17, "PredefinedType", Plain::Text("partitioning".into()))
        .unwrap();
    assert_eq!(
        model.get_value(17, "PredefinedType"),
        Ok(Selected::Value(Tagged::Enum("PARTITIONING".into())))
    );
    // The attribute changed; the facet still reads the type's predefined
    // type first, as IfcOpenShell does.
    assert_eq!(
        model.select("IfcWall, query:PredefinedType=PARTITIONING"),
        Ok(vec![17])
    );
    assert_eq!(
        model.select("IfcWall, PredefinedType=SOLIDWALL"),
        Ok(vec![17, 18, 19])
    );
}

#[cfg(feature = "properties-write")]
#[test]
fn a_seeded_write_by_path_reproduces_its_global_ids() {
    for (mut a, mut b) in fixtures().into_iter().zip(fixtures()) {
        let typed = || {
            Plain::Exact(Tagged::Typed {
                type_name: "IFCLABEL".into(),
                value: Box::new(Tagged::Text("x".into())),
            })
        };
        a.set_value_seeded(19, "Custom_Notes.Note", typed(), 559)
            .unwrap();
        b.set_value_seeded(19, "Custom_Notes.Note", typed(), 559)
            .unwrap();
        assert_eq!(a.write().unwrap(), b.write().unwrap());
    }
}

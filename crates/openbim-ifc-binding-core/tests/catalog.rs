//! The PSD/QTO catalog as the hosts load it (#318): embedded
//! (`property-catalog`, the C and Python bindings) or loaded at runtime
//! from pinned snapshot bytes (`property-catalog-runtime`, the npm
//! package). The gate runs this file with default features and with
//! `ifc4,property-catalog-runtime`.

use openbim_ifc_binding_core::catalog::{file_name, is_loaded, load, RELEASES};
use openbim_ifc_binding_core::property_edit::PropertyEdit;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::IfcModel;

/// Every edition's snapshot; its pin loads any edition it holds.
#[cfg_attr(not(feature = "property-catalog-runtime"), allow(dead_code))]
const CONTAINER: &[u8] = include_bytes!("../../ifc-template-catalog/data/catalog.bin");

#[cfg_attr(
    not(any(feature = "property-catalog", feature = "property-catalog-runtime")),
    allow(dead_code)
)]
fn fixture() -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

#[cfg_attr(
    not(any(feature = "property-catalog", feature = "property-catalog-runtime")),
    allow(dead_code)
)]
fn fire_rating(text: &str) -> PropertyEdit {
    PropertyEdit::set(
        30,
        "Pset_WallCommon",
        "FireRating",
        Tagged::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Tagged::Text(text.into())),
        },
    )
}

#[cfg(any(feature = "property-catalog", feature = "property-catalog-runtime"))]
#[test]
fn releases_name_their_files_and_others_are_refused() {
    assert_eq!(
        RELEASES.map(|release| file_name(release).unwrap()),
        ["ifc2x3-tc1.bin", "ifc4-add2-tc1.bin", "ifc4x3-add2.bin"]
    );
    assert_eq!(file_name("ifc4x3_add2").unwrap(), "ifc4x3-add2.bin");
    for release in ["IFC4X1", "IFC4X2", "IFC5", ""] {
        assert_eq!(file_name(release).unwrap_err().code(), "unsupported-schema");
        assert_eq!(is_loaded(release).unwrap_err().code(), "unsupported-schema");
        assert_eq!(
            load(release, CONTAINER).unwrap_err().code(),
            "unsupported-schema"
        );
    }
}

#[cfg(feature = "property-catalog")]
#[test]
fn an_embedded_catalog_needs_no_loading() {
    for release in RELEASES {
        assert!(is_loaded(release).unwrap());
        load(release, b"ignored: every edition is embedded").unwrap();
    }
    let mut model = fixture();
    model.set_properties(vec![fire_rating("F90")]).unwrap();
}

// One test: a loaded catalog is process-wide, so the order of the steps
// is the point.
#[cfg(all(
    feature = "property-catalog-runtime",
    not(feature = "property-catalog")
))]
#[test]
fn a_runtime_catalog_is_refused_until_loaded_then_checks_writes() {
    let mut model = fixture();
    let before = model.write().unwrap();
    assert!(!is_loaded("IFC4").unwrap());
    let error = model.set_properties(vec![fire_rating("F90")]).unwrap_err();
    assert_eq!(error.code(), "catalog-not-loaded");
    let message = error.to_string();
    assert!(message.contains("IFC4 ADD2 TC1"), "{message}");
    assert!(message.contains("loadCatalog"), "{message}");
    assert_eq!(
        model.write().unwrap(),
        before,
        "a refused write changed the model"
    );

    // Removal and a set without the prefix need no catalog.
    model
        .set_properties(vec![PropertyEdit::remove(
            30,
            "Pset_WallCommon",
            "IsExternal",
        )])
        .unwrap();

    // Bytes that are not the pinned snapshot load nothing.
    let mut damaged = CONTAINER.to_vec();
    damaged[100] ^= 1;
    assert_eq!(load("IFC4", &damaged).unwrap_err().code(), "invalid-value");
    assert_eq!(load("IFC4", b"").unwrap_err().code(), "invalid-value");
    assert!(!is_loaded("IFC4").unwrap());

    load("IFC4", CONTAINER).unwrap();
    assert!(is_loaded("ifc4").unwrap());
    assert!(!is_loaded("IFC4X3").unwrap());
    model.set_properties(vec![fire_rating("F90")]).unwrap();
    // Checked as with the embedded catalog: FireRating is an IfcLabel.
    let error = model
        .set_properties(vec![PropertyEdit::set(
            30,
            "Pset_WallCommon",
            "FireRating",
            Tagged::Typed {
                type_name: "IFCBOOLEAN".into(),
                value: Box::new(Tagged::Bool(true)),
            },
        )])
        .unwrap_err();
    assert_eq!(error.code(), "template-violation");
}

#[cfg(not(any(feature = "property-catalog", feature = "property-catalog-runtime")))]
#[test]
fn without_a_catalog_feature_loading_is_disabled() {
    assert_eq!(file_name("IFC4").unwrap_err().code(), "feature-disabled");
    assert_eq!(is_loaded("IFC4").unwrap_err().code(), "feature-disabled");
    assert_eq!(
        load("IFC4", CONTAINER).unwrap_err().code(),
        "feature-disabled"
    );
}

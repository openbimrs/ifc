//! Property edits against a catalog installed at runtime (#318): the
//! `property-catalog-runtime` build, without the embedded catalog.
//!
//! Until the release's edition is installed, a Set on a `Pset_`/`Qto_` set
//! is refused with `CatalogNotLoaded` and nothing is written; removal and
//! sets without the prefix need no catalog. Once installed, the edit is
//! checked exactly as with the embedded catalog. Installed catalogs are
//! process-wide, so the steps run in one test, in order.
//!
//! The gate runs this file with
//! `--features step,schema,properties,property-catalog-runtime`.

#![cfg(all(
    feature = "step",
    feature = "properties",
    feature = "property-catalog-runtime",
    not(feature = "property-catalog")
))]

use ifc::property_catalog::definition::CatalogEdition;
use ifc::property_catalog::runtime::{install, is_installed};
use ifc::{apply_property_edits, Codec, EntityId, Model, PropertyEdit, PropertyEditFailure};
use ifc::{StepCodec, Value};

/// Every edition's snapshot; its pin installs any edition it holds.
const CONTAINER: &[u8] = include_bytes!("../../ifc-template-catalog/data/catalog.bin");

fn fixture() -> Model {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-properties/synthetic_properties.ifc"
    );
    StepCodec
        .read_path(std::path::Path::new(path))
        .expect("fixture reads")
}

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

#[test]
fn a_catalog_set_waits_for_its_edition_then_is_checked() {
    let mut model = fixture();
    let before = StepCodec.write_bytes(&model).unwrap();
    let fire_rating = PropertyEdit::set(
        EntityId(30),
        "Pset_WallCommon",
        "FireRating",
        typed("IFCLABEL", Value::Text("F90".into())),
    );

    assert!(!is_installed(CatalogEdition::Ifc4Add2Tc1));
    let error = apply_property_edits(&mut model, std::slice::from_ref(&fire_rating))
        .expect_err("refused before loading");
    assert_eq!(
        error.failure,
        PropertyEditFailure::CatalogNotLoaded {
            set: "Pset_WallCommon".into(),
            edition: "IFC4 ADD2 TC1".into(),
        }
    );
    assert!(
        error
            .to_string()
            .contains("IFC4 ADD2 TC1 catalog is not installed"),
        "{error}"
    );
    assert_eq!(StepCodec.write_bytes(&model).unwrap(), before);

    // Another edition installed does not stand in for IFC4's.
    install(CatalogEdition::Ifc4x3Add2, CONTAINER).unwrap();
    assert!(matches!(
        apply_property_edits(&mut model, std::slice::from_ref(&fire_rating))
            .unwrap_err()
            .failure,
        PropertyEditFailure::CatalogNotLoaded { .. }
    ));

    // Removal and a set without the prefix need no catalog.
    apply_property_edits(
        &mut model,
        &[PropertyEdit::remove(
            EntityId(30),
            "Pset_WallCommon",
            "IsExternal",
        )],
    )
    .expect("removal needs no template");
    apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(31),
            "Acme_Notes",
            "Note",
            typed("IFCLABEL", Value::Text("x".into())),
        )],
    )
    .expect("an application set needs no template");

    install(CatalogEdition::Ifc4Add2Tc1, CONTAINER).unwrap();
    apply_property_edits(&mut model, std::slice::from_ref(&fire_rating))
        .expect("checked and written");
    // The template applies: FireRating is an IfcLabel, never a boolean.
    let refused = apply_property_edits(
        &mut model,
        &[PropertyEdit::set(
            EntityId(30),
            "Pset_WallCommon",
            "FireRating",
            typed("IFCBOOLEAN", Value::Bool(true)),
        )],
    )
    .expect_err("the template refuses");
    assert!(
        matches!(refused.failure, PropertyEditFailure::Template(_)),
        "{refused:?}"
    );
}

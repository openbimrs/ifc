//! The catalog a release reads (#566), as property edits and validation
//! read it: IFC2X3 as published, IFC4 and IFC4X3 corrected, no other release.
#![cfg(all(feature = "schema-api", feature = "property-catalog"))]

use ifc::property_catalog::catalog::CatalogProfile;
use ifc::property_catalog::definition::CatalogEdition;
use ifc::{release_catalog, ReleaseCatalogError, SchemaVersion};

fn read(version: SchemaVersion) -> (CatalogEdition, CatalogProfile) {
    let catalog = release_catalog(version).expect("the release has a catalog");
    (catalog.manifest().edition, catalog.profile())
}

#[test]
fn each_release_reads_its_edition_and_profile() {
    assert_eq!(
        read(SchemaVersion::Ifc2x3),
        (CatalogEdition::Ifc2x3Tc1, CatalogProfile::Official)
    );
    assert_eq!(
        read(SchemaVersion::Ifc4),
        (CatalogEdition::Ifc4Add2Tc1, CatalogProfile::Corrected)
    );
    assert_eq!(
        read(SchemaVersion::Ifc4x3),
        (CatalogEdition::Ifc4x3Add2, CatalogProfile::Corrected)
    );
}

#[test]
fn releases_without_a_catalog_are_refused() {
    for version in [SchemaVersion::Ifc4x1, SchemaVersion::Ifc4x2] {
        let error = release_catalog(version).unwrap_err();
        assert_eq!(error, ReleaseCatalogError::NoCatalog(version));
        assert!(error.to_string().contains("no PSD/QTO catalog"));
    }
}

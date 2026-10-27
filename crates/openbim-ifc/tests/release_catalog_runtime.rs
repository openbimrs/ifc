//! The release catalog in a runtime-catalog build (#566): refused until the
//! edition is installed, then read. Installed catalogs are process-wide, so
//! the steps run in one test, in order.
//!
//! The gate runs this file with
//! `--features step,schema,properties,property-catalog-runtime`.
#![cfg(all(
    feature = "schema-api",
    feature = "property-catalog-runtime",
    not(feature = "property-catalog")
))]

use ifc::property_catalog::catalog::CatalogProfile;
use ifc::property_catalog::definition::CatalogEdition;
use ifc::property_catalog::runtime::install;
use ifc::{release_catalog, ReleaseCatalogError, SchemaVersion};

const CONTAINER: &[u8] = include_bytes!("../../ifc-template-catalog/data/catalog.bin");

#[test]
fn a_release_waits_for_its_edition_then_reads_it() {
    assert_eq!(
        release_catalog(SchemaVersion::Ifc4).unwrap_err(),
        ReleaseCatalogError::NotInstalled {
            edition: CatalogEdition::Ifc4Add2Tc1
        }
    );
    // A release without a catalog is not an installation problem.
    assert_eq!(
        release_catalog(SchemaVersion::Ifc4x1).unwrap_err(),
        ReleaseCatalogError::NoCatalog(SchemaVersion::Ifc4x1)
    );

    install(CatalogEdition::Ifc4Add2Tc1, CONTAINER).expect("the pinned snapshot installs");
    let catalog = release_catalog(SchemaVersion::Ifc4).expect("installed");
    assert_eq!(catalog.manifest().edition, CatalogEdition::Ifc4Add2Tc1);
    assert_eq!(catalog.profile(), CatalogProfile::Corrected);
    // Another edition is still not installed.
    assert!(matches!(
        release_catalog(SchemaVersion::Ifc4x3),
        Err(ReleaseCatalogError::NotInstalled { .. })
    ));
}

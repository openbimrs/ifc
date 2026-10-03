//! Catalogs installed at runtime from pinned snapshot bytes (#318).

#![cfg(feature = "runtime")]

use ifc_template_catalog::catalog::CatalogProfile;
use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};
use ifc_template_catalog::runtime::{install, is_installed, load_catalog, RuntimeCatalogError};
use ifc_template_catalog::snapshot::{encode, pinned_sha256};

fn file(edition: CatalogEdition) -> Vec<u8> {
    encode(&[&official_catalog(edition).unwrap()]).unwrap()
}

// One test: installed catalogs are process-wide, so the order of these
// steps is the point.
#[test]
fn install_checks_the_pin_and_then_serves_both_profiles() {
    let ifc4 = CatalogEdition::Ifc4Add2Tc1;
    let ifc4x3 = CatalogEdition::Ifc4x3Add2;

    assert!(!is_installed(ifc4));
    assert_eq!(
        load_catalog(ifc4, CatalogProfile::Corrected).unwrap_err(),
        RuntimeCatalogError::NotInstalled(ifc4)
    );

    // Another edition's file, a damaged file and arbitrary bytes are
    // refused before anything is decoded.
    let error = install(ifc4, &file(ifc4x3)).unwrap_err();
    assert!(
        matches!(&error, RuntimeCatalogError::DigestMismatch { expected, .. } if *expected == pinned_sha256(ifc4)),
        "{error:?}"
    );
    let mut damaged = file(ifc4);
    *damaged.last_mut().unwrap() ^= 1;
    assert!(matches!(
        install(ifc4, &damaged),
        Err(RuntimeCatalogError::DigestMismatch { .. })
    ));
    assert!(install(ifc4, b"not a catalog").is_err());
    assert!(!is_installed(ifc4));

    install(ifc4, &file(ifc4)).unwrap();
    assert!(is_installed(ifc4));
    assert!(!is_installed(ifc4x3));
    // Installing again is a no-op, still checked.
    install(ifc4, &file(ifc4)).unwrap();
    assert!(install(ifc4, &damaged).is_err());

    let official = load_catalog(ifc4, CatalogProfile::Official).unwrap();
    let embedded = official_catalog(ifc4).unwrap();
    assert_eq!(format!("{official:?}"), format!("{embedded:?}"));
    let corrected = load_catalog(ifc4, CatalogProfile::Corrected).unwrap();
    let embedded = corrected_catalog(ifc4).unwrap();
    assert_eq!(format!("{corrected:?}"), format!("{embedded:?}"));

    // The container's pin installs any edition it holds.
    let container = include_bytes!("../data/catalog.bin");
    install(ifc4x3, container).unwrap();
    assert!(is_installed(ifc4x3));

    let ifc2x3 = CatalogEdition::Ifc2x3Tc1;
    install(ifc2x3, &file(ifc2x3)).unwrap();
    assert_eq!(
        load_catalog(ifc2x3, CatalogProfile::Corrected).unwrap_err(),
        RuntimeCatalogError::UnavailableEdition(ifc2x3)
    );
}

#![cfg(feature = "property-catalog")]

#[test]
fn facade_exposes_catalog_without_authored_properties() {
    let catalog = ifc::property_catalog::embedded::official_catalog(
        ifc::property_catalog::definition::CatalogEdition::Ifc4Add2Tc1,
    )
    .unwrap();
    assert_eq!(catalog.len(), 513);
    assert!(ifc::compiled_features().contains(&"property-catalog"));
}

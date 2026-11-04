//! Integration tests for [`ifc_template_catalog::embedded::corrected_catalog`].

use ifc_template_catalog::catalog::CatalogProfile;
use ifc_template_catalog::definition::{
    CatalogEdition, PropertyKind, QuantityKind, SetTemplateKind,
};
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};

#[test]
fn corrected_profile_is_explicit_and_official_snapshot_stays_unchanged() {
    let official = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let corrected = corrected_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    assert_eq!(official.profile(), CatalogProfile::Official);
    assert_eq!(corrected.profile(), CatalogProfile::Corrected);
    assert_eq!(corrected.applied_patches().len(), 8);
    let official_qto = official.get("Qto_WallBaseQuantities").unwrap();
    let corrected_qto = corrected.get("Qto_WallBaseQuantities").unwrap();
    assert_eq!(official_qto.applicability.len(), 1);
    assert_eq!(corrected_qto.applicability.len(), 2);
    assert_eq!(
        corrected
            .advisories_for("Pset_EnvironmentalImpactValues")
            .len(),
        1
    );
}

fn stationing_members(catalog: &ifc_template_catalog::catalog::Catalog) -> Vec<(String, String)> {
    let SetTemplateKind::Property { properties, .. } = &catalog
        .get("Pset_Stationing")
        .expect("Pset_Stationing")
        .kind
    else {
        panic!("Pset_Stationing is a property set")
    };
    properties
        .iter()
        .map(|property| {
            let PropertyKind::SingleValue { data_type } = &property.kind else {
                panic!("{} is a single value", property.name)
            };
            (
                property.name.clone(),
                data_type.type_name.clone().unwrap_or_default(),
            )
        })
        .collect()
}

/// IFC 4.3.2.0 (IFC4X3 ADD2) documentation, 6.6.4.10, Table 6.6.4.10.A lists
/// three members; the PSD XML the official snapshot is built from lists two.
#[test]
fn ifc4x3_corrected_profile_adds_the_documented_stationing_member() {
    let official = official_catalog(CatalogEdition::Ifc4x3Add2).unwrap();
    let corrected = corrected_catalog(CatalogEdition::Ifc4x3Add2).unwrap();
    assert_eq!(corrected.profile(), CatalogProfile::Corrected);
    assert_eq!(corrected.applied_patches().len(), 20);
    let pair = |name: &str, ty: &str| (name.to_owned(), ty.to_owned());
    assert_eq!(
        stationing_members(&official),
        [
            pair("IncomingStation", "IfcLengthMeasure"),
            pair("Station", "IfcLengthMeasure"),
        ]
    );
    assert_eq!(
        stationing_members(&corrected),
        [
            pair("IncomingStation", "IfcLengthMeasure"),
            pair("Station", "IfcLengthMeasure"),
            pair("HasIncreasingStation", "IfcBoolean"),
        ]
    );
}

fn proxy_quantity_kinds(
    catalog: &ifc_template_catalog::catalog::Catalog,
) -> Vec<(String, QuantityKind)> {
    let SetTemplateKind::Quantity { quantities, .. } = &catalog
        .get("Qto_BuildingElementProxyQuantities")
        .expect("Qto_BuildingElementProxyQuantities")
        .kind
    else {
        panic!("Qto_BuildingElementProxyQuantities is a quantity set")
    };
    quantities
        .iter()
        .map(|quantity| (quantity.name.clone(), quantity.kind))
        .collect()
}

/// The source PSD XML declares both members `Q_COUNT` in IFC4 ADD2 TC1 and
/// IFC4X3 ADD2; a measured area and volume are what files carry (#553).
#[test]
fn the_proxy_quantities_are_an_area_and_a_volume_in_the_corrected_profiles() {
    for edition in [CatalogEdition::Ifc4Add2Tc1, CatalogEdition::Ifc4x3Add2] {
        let official = official_catalog(edition).unwrap();
        let corrected = corrected_catalog(edition).unwrap();
        assert_eq!(
            proxy_quantity_kinds(&official),
            [
                ("NetSurfaceArea".to_owned(), QuantityKind::Count),
                ("NetVolume".to_owned(), QuantityKind::Count),
            ],
            "{edition:?}"
        );
        assert_eq!(
            proxy_quantity_kinds(&corrected),
            [
                ("NetSurfaceArea".to_owned(), QuantityKind::Area),
                ("NetVolume".to_owned(), QuantityKind::Volume),
            ],
            "{edition:?}"
        );
    }
}

fn process_item(catalog: &ifc_template_catalog::catalog::Catalog, set: &str) -> PropertyKind {
    let SetTemplateKind::Property { properties, .. } = &catalog.get(set).expect(set).kind else {
        panic!("{set} is a property set")
    };
    properties
        .iter()
        .find(|property| property.name == "ProcessItem")
        .unwrap_or_else(|| panic!("{set} has no ProcessItem"))
        .kind
        .clone()
}

/// The IFC4X3 PSD XML declares `ProcessItem` a single value of the data type
/// `PEnum_ProcessItem`, an enumeration of six items (#553).
#[test]
fn process_item_is_an_enumerated_value_in_the_corrected_ifc4x3_profile() {
    let official = official_catalog(CatalogEdition::Ifc4x3Add2).unwrap();
    let corrected = corrected_catalog(CatalogEdition::Ifc4x3Add2).unwrap();
    for set in ["Pset_CargoCommon", "Pset_ProcessCapacity"] {
        assert!(
            matches!(process_item(&official, set), PropertyKind::SingleValue { data_type }
                if data_type.type_name.as_deref() == Some("PEnum_ProcessItem")),
            "{set}"
        );
        let PropertyKind::EnumeratedValue {
            enumeration_name,
            values,
            ..
        } = process_item(&corrected, set)
        else {
            panic!("{set}: ProcessItem is not enumerated")
        };
        assert_eq!(
            enumeration_name.as_deref(),
            Some("PEnum_ProcessItem"),
            "{set}"
        );
        assert_eq!(
            values,
            ["TEU", "CGT", "BARREL", "TONNE", "PASSENGER", "VEHICLE"],
            "{set}"
        );
    }
}

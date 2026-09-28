//! Integration tests for [`ifc_template_catalog::embedded::corrected_catalog`].

use ifc_template_catalog::catalog::CatalogProfile;
use ifc_template_catalog::definition::{CatalogEdition, PropertyKind, SetTemplateKind};
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};

#[test]
fn corrected_profile_is_explicit_and_official_snapshot_stays_unchanged() {
    let official = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let corrected = corrected_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    assert_eq!(official.profile(), CatalogProfile::Official);
    assert_eq!(corrected.profile(), CatalogProfile::Corrected);
    assert_eq!(corrected.applied_patches().len(), 3);
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
    assert_eq!(corrected.applied_patches().len(), 1);
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

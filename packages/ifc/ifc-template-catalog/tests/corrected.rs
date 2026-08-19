use ifc_template_catalog::catalog::CatalogProfile;
use ifc_template_catalog::definition::CatalogEdition;
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

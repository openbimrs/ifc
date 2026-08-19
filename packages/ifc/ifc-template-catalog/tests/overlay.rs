#[path = "support/mod.rs"]
mod support;

use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::definition::{Applicability, CatalogEdition};
use ifc_template_catalog::overlay::{AdvisorySeverity, Patch, PatchError, PatchOperation};
use support::{manifest, property_set};

fn add_type_patch(id: &str) -> Patch {
    Patch {
        id: id.into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: "Qto_WallBaseQuantities".into(),
        rationale: "backport type applicability".into(),
        evidence: "IfcOpenShell test/util/test_pset.py".into(),
        operation: PatchOperation::AddApplicability(Applicability::entity("IfcWallType")),
    }
}

#[test]
fn overlays_create_a_new_snapshot_and_preserve_official_data() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::entity("IfcWall")];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();

    let corrected = official
        .with_patches(
            CatalogProfile::Corrected,
            &[add_type_patch("NEH-IFC4-QTO-0001")],
        )
        .unwrap();

    assert_eq!(
        official
            .get("Qto_WallBaseQuantities")
            .unwrap()
            .applicability
            .len(),
        1
    );
    assert_eq!(
        corrected
            .get("Qto_WallBaseQuantities")
            .unwrap()
            .applicability
            .len(),
        2
    );
    assert_eq!(corrected.applied_patches()[0].id, "NEH-IFC4-QTO-0001");
}

#[test]
fn stale_or_duplicate_corrections_fail_loudly() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::entity("IfcWallType")];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();

    let error = official
        .with_patches(
            CatalogProfile::Corrected,
            &[add_type_patch("NEH-IFC4-QTO-0001")],
        )
        .unwrap_err();
    assert!(matches!(error, PatchError::AlreadyApplied { .. }));
}

#[test]
fn add_then_replace_applicability_is_a_conflict() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::entity("IfcWall")];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();
    let replace = Patch {
        id: "replace".into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: "Qto_WallBaseQuantities".into(),
        rationale: "fixture".into(),
        evidence: "fixture".into(),
        operation: PatchOperation::ReplaceApplicability {
            expected: vec![
                Applicability::entity("IfcWall"),
                Applicability::entity("IfcWallType"),
            ],
            replacement: vec![Applicability::entity("IfcBuildingElement")],
        },
    };
    let error = official
        .with_patches(CatalogProfile::Custom, &[add_type_patch("add"), replace])
        .unwrap_err();
    assert!(matches!(error, PatchError::ConflictingApplicability { .. }));
}

#[test]
fn advisories_are_provenance_bearing_and_do_not_rewrite_templates() {
    let pset = property_set("Pset_EnvironmentalImpactValues");
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![pset]).unwrap();
    let patch = Patch {
        id: "NEH-IFC4-EPD-0001".into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: "Pset_EnvironmentalImpactValues".into(),
        rationale: "legacy scalar model cannot represent an EPD module matrix".into(),
        evidence: "ADR 0010".into(),
        operation: PatchOperation::AddAdvisory {
            severity: AdvisorySeverity::Warning,
            message: "Legacy and underspecified for module-based EPD data".into(),
        },
    };

    let corrected = official
        .with_patches(CatalogProfile::Corrected, &[patch])
        .unwrap();
    let advisories = corrected.advisories_for("Pset_EnvironmentalImpactValues");
    assert_eq!(advisories.len(), 1);
    assert_eq!(advisories[0].patch_id, "NEH-IFC4-EPD-0001");
}

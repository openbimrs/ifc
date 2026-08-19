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

fn replace_patch(id: &str) -> Patch {
    Patch {
        id: id.into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: "Qto_WallBaseQuantities".into(),
        rationale: "replace fixture applicability".into(),
        evidence: "fixture".into(),
        operation: PatchOperation::ReplaceApplicability {
            expected: vec![
                Applicability::entity("IfcWall"),
                Applicability::entity("IfcWallType"),
            ],
            replacement: vec![Applicability::entity("IfcBuildingElement")],
        },
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
    assert_eq!(
        corrected.applied_patches()[0].rationale,
        "backport type applicability"
    );
    assert!(matches!(
        corrected.applied_patches()[0].operation,
        PatchOperation::AddApplicability(_)
    ));
}

#[test]
fn overlays_cannot_relabel_or_create_empty_corrected_snapshots() {
    let official = Catalog::try_new(
        manifest(1, 0),
        CatalogProfile::Official,
        vec![property_set("Qto_WallBaseQuantities")],
    )
    .unwrap();
    assert!(matches!(
        official.with_patches(CatalogProfile::Official, &[add_type_patch("add")]),
        Err(PatchError::InvalidProfileTransition { .. })
    ));
    assert!(matches!(
        official.with_patches(CatalogProfile::Corrected, &[]),
        Err(PatchError::EmptyLedger)
    ));
}

#[test]
fn stale_or_duplicate_corrections_fail_loudly() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::entity("ifcwalltype")];
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
    let replace = replace_patch("replace");
    let error = official
        .with_patches(CatalogProfile::Custom, &[add_type_patch("add"), replace])
        .unwrap_err();
    assert!(matches!(error, PatchError::ConflictingApplicability { .. }));
}

#[test]
fn applicability_conflicts_survive_separate_overlay_calls() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::entity("IfcWall")];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();
    let corrected = official
        .with_patches(CatalogProfile::Corrected, &[add_type_patch("add")])
        .unwrap();
    let error = corrected
        .with_patches(CatalogProfile::Custom, &[replace_patch("replace")])
        .unwrap_err();
    assert!(matches!(error, PatchError::ConflictingApplicability { .. }));
}

#[test]
fn replace_conflicts_survive_separate_overlay_calls() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![
        Applicability::entity("IfcWall"),
        Applicability::entity("IfcWallType"),
    ];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();
    let corrected = official
        .with_patches(CatalogProfile::Corrected, &[replace_patch("replace-1")])
        .unwrap();

    for patch in [add_type_patch("add"), replace_patch("replace-2")] {
        assert!(matches!(
            corrected.with_patches(CatalogProfile::Custom, &[patch]),
            Err(PatchError::ConflictingApplicability { .. })
        ));
    }
}

#[test]
fn applicability_dedup_is_case_insensitive_for_predefined_types() {
    let mut qto = property_set("Qto_WallBaseQuantities");
    qto.applicability = vec![Applicability::parse("IfcWall/USERDEFINED").unwrap()];
    let official = Catalog::try_new(manifest(1, 0), CatalogProfile::Official, vec![qto]).unwrap();
    let mut duplicate = add_type_patch("duplicate");
    duplicate.operation =
        PatchOperation::AddApplicability(Applicability::parse("ifcwall/userdefined").unwrap());

    assert!(matches!(
        official.with_patches(CatalogProfile::Corrected, &[duplicate]),
        Err(PatchError::AlreadyApplied { .. })
    ));
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

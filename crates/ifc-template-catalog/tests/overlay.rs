//! Integration tests for [`ifc_template_catalog::catalog::Catalog::with_patches`] and corrections.

#[path = "support/mod.rs"]
mod support;
use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::definition::{
    Applicability, CatalogEdition, PropertyDataType, PropertyKind, PropertyTemplate, QuantityKind,
    QuantitySetType, SetTemplateKind,
};
use ifc_template_catalog::embedded::official_catalog;
use ifc_template_catalog::overlay::{AdvisorySeverity, Patch, PatchError, PatchOperation};
use support::{manifest, property_set, quantity_set};

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
        evidence: "ADR 0017".into(),
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

fn add_property_patch(id: &str, target: &str) -> Patch {
    Patch {
        id: id.into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: target.into(),
        rationale: "fixture".into(),
        evidence: "fixture".into(),
        operation: PatchOperation::AddProperty(PropertyTemplate {
            name: "Flag".into(),
            guid: None,
            definition: None,
            name_aliases: Vec::new(),
            definition_aliases: Vec::new(),
            kind: PropertyKind::SingleValue {
                data_type: PropertyDataType::new("IfcBoolean"),
            },
        }),
    }
}

#[test]
fn an_added_property_is_appended_once_and_only_to_a_property_set() {
    let official = Catalog::try_new(
        manifest(1, 1),
        CatalogProfile::Official,
        vec![
            property_set("Pset_Test"),
            quantity_set("Qto_Test", QuantitySetType::Unspecified),
        ],
    )
    .unwrap();
    let corrected = official
        .with_patches(
            CatalogProfile::Corrected,
            &[add_property_patch("P-1", "Pset_Test")],
        )
        .unwrap();
    let SetTemplateKind::Property { properties, .. } = &corrected.get("Pset_Test").unwrap().kind
    else {
        panic!()
    };
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].name, "Flag");
    assert!(matches!(
        corrected.with_patches(
            CatalogProfile::Corrected,
            &[add_property_patch("P-2", "Pset_Test")]
        ),
        Err(PatchError::InvalidProfileTransition { .. })
    ));
    assert!(matches!(
        official.with_patches(
            CatalogProfile::Corrected,
            &[
                add_property_patch("P-1", "Pset_Test"),
                add_property_patch("P-2", "Pset_Test")
            ]
        ),
        Err(PatchError::AlreadyApplied { .. })
    ));
    assert!(matches!(
        official.with_patches(
            CatalogProfile::Corrected,
            &[add_property_patch("P-1", "Qto_Test")]
        ),
        Err(PatchError::NotAPropertySet { .. })
    ));
}

fn replace_quantity_patch(target: &str, member: &str, expected: QuantityKind) -> Patch {
    Patch {
        id: "Q-1".into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: target.into(),
        rationale: "fixture".into(),
        evidence: "fixture".into(),
        operation: PatchOperation::ReplaceQuantityKind {
            member: member.into(),
            expected,
            replacement: QuantityKind::Volume,
        },
    }
}

fn replace_property_patch(target: &str, member: &str) -> Patch {
    Patch {
        id: "P-1".into(),
        edition: CatalogEdition::Ifc4Add2Tc1,
        target_template: target.into(),
        rationale: "fixture".into(),
        evidence: "fixture".into(),
        operation: PatchOperation::ReplacePropertyKind {
            member: member.into(),
            expected: PropertyKind::SingleValue {
                data_type: PropertyDataType::new("IfcBoolean"),
            },
            replacement: PropertyKind::ListValue {
                data_type: PropertyDataType::new("IfcBoolean"),
            },
        },
    }
}

#[test]
fn a_replaced_member_kind_needs_the_expected_kind_a_set_of_that_family_and_the_member() {
    let official = official_catalog(CatalogEdition::Ifc4Add2Tc1).unwrap();
    let apply = |patch| official.with_patches(CatalogProfile::Corrected, &[patch]);
    let wall = "Qto_WallBaseQuantities";

    let corrected = apply(replace_quantity_patch(wall, "Length", QuantityKind::Length)).unwrap();
    let SetTemplateKind::Quantity { quantities, .. } = &corrected.get(wall).unwrap().kind else {
        panic!()
    };
    let kind = |name: &str| quantities.iter().find(|q| q.name == name).unwrap().kind;
    assert_eq!(kind("Length"), QuantityKind::Volume);
    assert_eq!(kind("Width"), QuantityKind::Length);

    assert!(matches!(
        apply(replace_quantity_patch(wall, "Length", QuantityKind::Count)),
        Err(PatchError::StaleTarget { .. })
    ));
    assert!(matches!(
        apply(replace_quantity_patch(wall, "Nope", QuantityKind::Length)),
        Err(PatchError::UnknownMember { .. })
    ));
    assert!(matches!(
        apply(replace_quantity_patch(
            "Pset_WallCommon",
            "IsExternal",
            QuantityKind::Length
        )),
        Err(PatchError::NotAQuantitySet { .. })
    ));

    let corrected = apply(replace_property_patch("Pset_WallCommon", "IsExternal")).unwrap();
    let SetTemplateKind::Property { properties, .. } =
        &corrected.get("Pset_WallCommon").unwrap().kind
    else {
        panic!()
    };
    let flag = properties.iter().find(|p| p.name == "IsExternal").unwrap();
    assert!(matches!(flag.kind, PropertyKind::ListValue { .. }));
    assert!(matches!(
        apply(replace_property_patch("Pset_WallCommon", "Reference")),
        Err(PatchError::StaleTarget { .. })
    ));
    assert!(matches!(
        apply(replace_property_patch("Pset_WallCommon", "Nope")),
        Err(PatchError::UnknownMember { .. })
    ));
    assert!(matches!(
        apply(replace_property_patch(wall, "Length")),
        Err(PatchError::NotAPropertySet { .. })
    ));
}

//! Sets the published sources leave without an applicable entity, and the
//! corrected profile's evidence-backed fills (#622).

use ifc_template_catalog::definition::{Applicability, CatalogEdition};
use ifc_template_catalog::diagnostic::DiagnosticCode;
use ifc_template_catalog::embedded::{corrected_catalog, official_catalog};
use ifc_template_catalog::overlay::{corrected_patches, PatchOperation};
use ifc_template_catalog::query::{
    ApplicabilityContext, ApplicabilityTarget, ExactEntityHierarchy,
};

const IFC4: CatalogEdition = CatalogEdition::Ifc4Add2Tc1;
const IFC4X3: CatalogEdition = CatalogEdition::Ifc4x3Add2;

const IFC4_EMPTY: [&str; 4] = [
    "Pset_CivilElementCommon",
    "Pset_ElectricFlowStorageDevicePHistory",
    "Pset_ElementAssemblyCommon",
    "Pset_SpatialZoneCommon",
];

const IFC4X3_EMPTY: [&str; 15] = [
    "Pset_CivilElementCommon",
    "Pset_ElementAssemblyCommon",
    "Pset_MarineFacilityTransportation",
    "Pset_SpatialZoneCommon",
    "Qto_CourseBaseQuantities",
    "Qto_EarthworksCutBaseQuantities",
    "Qto_EarthworksFillBaseQuantities",
    "Qto_ImpactProtectionDeviceBaseQuantities",
    "Qto_KerbBaseQuantities",
    "Qto_PavementBaseQuantities",
    "Qto_PictorialSignQuantities",
    "Qto_ReinforcedSoilBaseQuantities",
    "Qto_SignBaseQuantities",
    "Qto_SignalBaseQuantities",
    "Qto_SurfaceFeatureBaseQuantities",
];

/// (edition, set, entity, predefined type) of every filled gap.
const FILLS: [(CatalogEdition, &str, &str, Option<&str>); 18] = [
    (IFC4, "Pset_CivilElementCommon", "IfcCivilElement", None),
    (
        IFC4,
        "Pset_ElementAssemblyCommon",
        "IfcElementAssembly",
        None,
    ),
    (IFC4, "Pset_SpatialZoneCommon", "IfcSpatialZone", None),
    (IFC4X3, "Pset_CivilElementCommon", "IfcCivilElement", None),
    (
        IFC4X3,
        "Pset_ElementAssemblyCommon",
        "IfcElementAssembly",
        None,
    ),
    (
        IFC4X3,
        "Pset_MarineFacilityTransportation",
        "IfcMarineFacility",
        None,
    ),
    (IFC4X3, "Pset_SpatialZoneCommon", "IfcSpatialZone", None),
    (IFC4X3, "Qto_CourseBaseQuantities", "IfcCourse", None),
    (
        IFC4X3,
        "Qto_EarthworksCutBaseQuantities",
        "IfcEarthworksCut",
        None,
    ),
    (
        IFC4X3,
        "Qto_EarthworksFillBaseQuantities",
        "IfcEarthworksFill",
        None,
    ),
    (
        IFC4X3,
        "Qto_ImpactProtectionDeviceBaseQuantities",
        "IfcImpactProtectionDevice",
        None,
    ),
    (IFC4X3, "Qto_KerbBaseQuantities", "IfcKerb", None),
    (IFC4X3, "Qto_PavementBaseQuantities", "IfcPavement", None),
    (
        IFC4X3,
        "Qto_PictorialSignQuantities",
        "IfcSign",
        Some("PICTORAL"),
    ),
    (
        IFC4X3,
        "Qto_ReinforcedSoilBaseQuantities",
        "IfcReinforcedSoil",
        None,
    ),
    (IFC4X3, "Qto_SignBaseQuantities", "IfcSign", None),
    (IFC4X3, "Qto_SignalBaseQuantities", "IfcSignal", None),
    (
        IFC4X3,
        "Qto_SurfaceFeatureBaseQuantities",
        "IfcSurfaceFeature",
        None,
    ),
];

fn empty_names(catalog: &ifc_template_catalog::catalog::Catalog) -> Vec<String> {
    let mut names: Vec<String> = catalog
        .diagnostics()
        .into_iter()
        .filter(|item| item.code == DiagnosticCode::EmptyApplicability)
        .map(|item| item.template)
        .collect();
    names.sort();
    names
}

#[test]
fn official_empty_applicability_is_exactly_what_the_source_publishes() {
    let ifc2x3 = official_catalog(CatalogEdition::Ifc2x3Tc1).unwrap();
    assert!(empty_names(&ifc2x3).is_empty());
    for (edition, expected) in [(IFC4, &IFC4_EMPTY[..]), (IFC4X3, &IFC4X3_EMPTY[..])] {
        let official = official_catalog(edition).unwrap();
        assert_eq!(empty_names(&official), expected, "{edition:?}");
        for name in expected {
            let template = official.get(name).unwrap();
            assert!(template.raw_applicability.is_none(), "{name}");
            assert!(template.applicability.is_empty(), "{name}");
        }
    }
}

#[test]
fn corrected_profiles_fill_every_gap_with_evidence() {
    let ifc4 = corrected_catalog(IFC4).unwrap();
    assert_eq!(
        empty_names(&ifc4),
        ["Pset_ElectricFlowStorageDevicePHistory"]
    );
    let ifc4x3 = corrected_catalog(IFC4X3).unwrap();
    assert!(empty_names(&ifc4x3).is_empty());
    for edition in [IFC4, IFC4X3] {
        for patch in corrected_patches(edition) {
            if matches!(patch.operation, PatchOperation::ReplaceApplicability { .. }) {
                assert!(!patch.evidence.is_empty(), "{}", patch.id);
                assert!(!patch.rationale.is_empty(), "{}", patch.id);
            }
        }
    }
}

#[test]
fn each_gap_gets_the_entity_its_evidence_names() {
    for (edition, set, entity, predefined) in FILLS {
        let official = official_catalog(edition).unwrap();
        let corrected = corrected_catalog(edition).unwrap();
        assert!(official.get(set).unwrap().applicability.is_empty(), "{set}");
        let applicability = &corrected.get(set).unwrap().applicability;
        assert_eq!(applicability.len(), 1, "{edition:?} {set}");
        let selector: &Applicability = &applicability[0];
        let raw = predefined.map_or(entity.to_owned(), |ty| format!("{entity}/{ty}"));
        assert_eq!(selector.raw, raw, "{edition:?} {set}");
        assert_eq!(selector.entity, entity, "{edition:?} {set}");
        assert_eq!(selector.predefined_type.as_deref(), predefined, "{set}");
    }
}

#[test]
fn element_assembly_common_matches_an_assembly() {
    let target = ApplicabilityTarget::new("IfcElementAssembly", Some("USERDEFINED"))
        .with_context(ApplicabilityContext::Occurrence);
    for edition in [IFC4, IFC4X3] {
        let corrected = corrected_catalog(edition).unwrap();
        let names: Vec<_> = corrected
            .applicable_to(&target, &ExactEntityHierarchy)
            .into_iter()
            .map(|template| template.name.as_str())
            .collect();
        assert!(names.contains(&"Pset_ElementAssemblyCommon"), "{edition:?}");
    }
}

#[test]
fn pictorial_sign_quantities_match_only_pictorial_signs() {
    let corrected = corrected_catalog(IFC4X3).unwrap();
    let matches = |predefined: &str| {
        let target = ApplicabilityTarget::new("IfcSign", Some(predefined))
            .with_context(ApplicabilityContext::Occurrence);
        corrected
            .applicable_to(&target, &ExactEntityHierarchy)
            .into_iter()
            .any(|template| template.name == "Qto_PictorialSignQuantities")
    };
    assert!(matches("PICTORAL"));
    assert!(!matches("MARKER"));
}

#[test]
fn built_in_applicability_patches_only_fill_published_gaps() {
    for (edition, empty) in [(IFC4, &IFC4_EMPTY[..]), (IFC4X3, &IFC4X3_EMPTY[..])] {
        let mut replaced = 0;
        for patch in corrected_patches(edition) {
            if let PatchOperation::ReplaceApplicability { expected, .. } = &patch.operation {
                replaced += 1;
                assert!(expected.is_empty(), "{}", patch.id);
                assert!(
                    empty.contains(&patch.target_template.as_str()),
                    "{}",
                    patch.id
                );
            }
        }
        assert!(replaced > 0, "{edition:?}");
    }
}

//! Evidence-backed Nehirde correction ledger.

use crate::definition::{
    Applicability, CatalogEdition, PropertyDataType, PropertyKind, PropertyTemplate,
};

use super::{AdvisorySeverity, Patch, PatchOperation};

/// Ordered built-in patches for an exact source edition.
pub fn corrected_patches(edition: CatalogEdition) -> Vec<Patch> {
    match edition {
        CatalogEdition::Ifc4Add2Tc1 => {
            vec![
            Patch {
                id: "NEH-IFC4-QTO-0001".into(),
                edition,
                target_template: "Qto_WallBaseQuantities".into(),
                rationale: "Backport the type applicability published by later catalogs".into(),
                evidence: "IfcOpenShell test/util/test_pset.py: backported IFC4 fix".into(),
                operation: PatchOperation::AddApplicability(Applicability::entity("IfcWallType")),
            },
            environmental_advisory(
                edition,
                "NEH-IFC4-EPD-0001",
                "Pset_EnvironmentalImpactIndicators",
            ),
            environmental_advisory(
                edition,
                "NEH-IFC4-EPD-0002",
                "Pset_EnvironmentalImpactValues",
            ),
            applicability_from_evidence(
                edition,
                "NEH-IFC4-PSD-0001",
                "Pset_CivilElementCommon",
                Applicability::entity("IfcCivilElement"),
                "IFC4 ADD2 TC1 publishes this set with no applicable entity; backport the entity \
                 later evidence names",
                "IFC4 ADD2 TC1 psd/Pset_CivilElementCommon.xml and \
                 annex-a/general-usage/IFC4_ADD2.ifc #541 declare no applicable entity; \
                 IFC4X3 ADD2 associates the same template GUID with IfcCivilElement \
                 (IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
                 schemas/IFC.xml, association EAID_106DE783_E427_4cae_91F7_626CDCE4D04B); \
                 IfcOpenShell 1a6336bd Pset_IFC4_ADD2.ifc #541: 'IfcCivilElement'".into(),
            ),
            applicability_from_evidence(
                edition,
                "NEH-IFC4-PSD-0002",
                "Pset_ElementAssemblyCommon",
                Applicability::entity("IfcElementAssembly"),
                "IFC4 ADD2 TC1 publishes this set with no applicable entity; backport the entity \
                 later evidence names",
                "IFC4 ADD2 TC1 psd/Pset_ElementAssemblyCommon.xml and \
                 annex-a/general-usage/IFC4_ADD2.ifc #559 declare no applicable entity; \
                 IFC4X3 ADD2 associates the same template GUID with IfcElementAssembly \
                 (IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
                 schemas/IFC.xml, association EAID_097D723E_5187_42d7_AAC0_0F29E0733215); \
                 IfcOpenShell 1a6336bd Pset_IFC4_ADD2.ifc #559: 'IfcElementAssembly'".into(),
            ),
            applicability_from_evidence(
                edition,
                "NEH-IFC4-PSD-0003",
                "Pset_SpatialZoneCommon",
                Applicability::entity("IfcSpatialZone"),
                "IFC4 ADD2 TC1 publishes this set with no applicable entity; backport the entity \
                 later evidence names",
                "IFC4 ADD2 TC1 psd/Pset_SpatialZoneCommon.xml and \
                 annex-a/general-usage/IFC4_ADD2.ifc #1750 declare no applicable entity; \
                 IFC4X3 ADD2 associates the same template GUID with IfcSpatialZone \
                 (IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
                 schemas/IFC.xml, association EAID_239365BE_FADD_46bf_A54E_3FA8D892187C); \
                 IfcOpenShell 1a6336bd Pset_IFC4_ADD2.ifc #1750: 'IfcSpatialZone'".into(),
            ),
        ]
        }
        CatalogEdition::Ifc4x3Add2 => vec![
        Patch {
            id: "NEH-IFC4X3-PSD-0001".into(),
            edition,
            target_template: "Pset_Stationing".into(),
            rationale: "The published ADD2 documentation lists HasIncreasingStation; the \
                        embedded reference_schemas PSD XML omits it"
                .into(),
            evidence: "IFC 4.3.2.0 (IFC4X3 ADD2) documentation, 6.6.4.10 Pset_Stationing, \
                       Table 6.6.4.10.A: HasIncreasingStation, IfcPropertySingleValue, IfcBoolean"
                .into(),
            // Name, form and type only: the documentation publishes no
            // GlobalId for the member, and its prose is not restated here.
            operation: PatchOperation::AddProperty(PropertyTemplate {
                name: "HasIncreasingStation".into(),
                guid: None,
                definition: None,
                name_aliases: Vec::new(),
                definition_aliases: Vec::new(),
                kind: PropertyKind::SingleValue {
                    data_type: PropertyDataType::new("IfcBoolean"),
                },
            }),
        },
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-PSD-0002",
            "Pset_CivilElementCommon",
            Applicability::entity("IfcCivilElement"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_106DE783_E427_4cae_91F7_626CDCE4D04B between \
             Pset_CivilElementCommon and IfcCivilElement".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-PSD-0003",
            "Pset_ElementAssemblyCommon",
            Applicability::entity("IfcElementAssembly"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_097D723E_5187_42d7_AAC0_0F29E0733215 between \
             Pset_ElementAssemblyCommon and IfcElementAssembly".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-PSD-0004",
            "Pset_MarineFacilityTransportation",
            Applicability::entity("IfcMarineFacility"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_61F9C3A1_FD78_4512_8432_BC3B5BAEEF8C between \
             Pset_MarineFacilityTransportation and IfcMarineFacility".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-PSD-0005",
            "Pset_SpatialZoneCommon",
            Applicability::entity("IfcSpatialZone"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_239365BE_FADD_46bf_A54E_3FA8D892187C between \
             Pset_SpatialZoneCommon and IfcSpatialZone".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0001",
            "Qto_CourseBaseQuantities",
            Applicability::entity("IfcCourse"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_C65F7735_7FB2_4847_B5C4_076476EF53D4 between \
             Qto_CourseBaseQuantities and IfcCourse".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0002",
            "Qto_EarthworksCutBaseQuantities",
            Applicability::entity("IfcEarthworksCut"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_00D7D326_5DEC_438d_AD40_E95B9285BD91 between \
             Qto_EarthworksCutBaseQuantities and IfcEarthworksCut".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0003",
            "Qto_EarthworksFillBaseQuantities",
            Applicability::entity("IfcEarthworksFill"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_2517FAD3_A946_42cb_8047_D8C2A0A1300A between \
             Qto_EarthworksFillBaseQuantities and IfcEarthworksFill".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0004",
            "Qto_ImpactProtectionDeviceBaseQuantities",
            Applicability::entity("IfcImpactProtectionDevice"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_DDD6466D_A2DD_4afa_A5A4_A50AB1401342 between \
             Qto_ImpactProtectionDeviceBaseQuantities and IfcImpactProtectionDevice".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0005",
            "Qto_KerbBaseQuantities",
            Applicability::entity("IfcKerb"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_9CB6B0D7_518E_41ac_BA77_33C47D5094BA between \
             Qto_KerbBaseQuantities and IfcKerb".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0006",
            "Qto_PavementBaseQuantities",
            Applicability::entity("IfcPavement"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_42C83610_A2B1_4530_9D13_CBB07161FAB0 between \
             Qto_PavementBaseQuantities and IfcPavement".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0007",
            "Qto_PictorialSignQuantities",
            Applicability {
                    raw: "IfcSign/PICTORAL".into(),
                    entity: "IfcSign".into(),
                    predefined_type: Some("PICTORAL".into()),
                },
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_708CD5AD_A390_486f_B4D7_E5B57307D48C between \
             Qto_PictorialSignQuantities and IfcSignTypeEnum.PICTORAL".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0008",
            "Qto_ReinforcedSoilBaseQuantities",
            Applicability::entity("IfcReinforcedSoil"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_64009BF7_8698_4797_920C_6DCD7C4D0820 between \
             Qto_ReinforcedSoilBaseQuantities and IfcReinforcedSoil".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0009",
            "Qto_SignBaseQuantities",
            Applicability::entity("IfcSign"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_9A2DD3BB_4D54_4875_BBEB_BB9B413B4D71 between \
             Qto_SignBaseQuantities and IfcSign".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0010",
            "Qto_SignalBaseQuantities",
            Applicability::entity("IfcSignal"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_4C85C5F9_B489_4139_BA94_188A5ABAB22E between \
             Qto_SignalBaseQuantities and IfcSignal".into(),
        ),
        applicability_from_evidence(
            edition,
            "NEH-IFC4X3-QTO-0011",
            "Qto_SurfaceFeatureBaseQuantities",
            Applicability::entity("IfcSurfaceFeature"),
            "The pinned reference_schemas PSD XML lists no applicable entity; schemas/IFC.xml at \
             the same commit associates the set with one",
            "buildingSMART IFC4.x-development 524daac53ca682e0649d240ace87f4cd7baff6e7 \
             schemas/IFC.xml: association EAID_B40B9136_EB56_4717_8EDB_9CD7FDE69A22 between \
             Qto_SurfaceFeatureBaseQuantities and IfcSurfaceFeature".into(),
        ),
        ],
        _ => Vec::new(),
    }
}

fn environmental_advisory(edition: CatalogEdition, id: &str, target: &str) -> Patch {
    Patch {
        id: id.into(),
        edition,
        target_template: target.into(),
        rationale: "Flag legacy scalar environmental data without changing official semantics".into(),
        evidence: "docs/adr/0017-versioned-psd-qto-catalog.md".into(),
        operation: PatchOperation::AddAdvisory {
            severity: AdvisorySeverity::Warning,
            message: "Legacy and underspecified for module-based EPD data; use an explicit EPD domain model"
                .into(),
        },
    }
}

/// Fill a set the source publishes with no applicable entity.
///
/// `expected` is the empty list on purpose: if a regenerated snapshot ever
/// gains a selector, the patch fails as stale instead of stacking a second
/// one on top of the published value.
fn applicability_from_evidence(
    edition: CatalogEdition,
    id: &str,
    target: &str,
    selector: Applicability,
    rationale: &str,
    evidence: String,
) -> Patch {
    Patch {
        id: id.into(),
        edition,
        target_template: target.into(),
        rationale: rationale.into(),
        evidence,
        operation: PatchOperation::ReplaceApplicability {
            expected: Vec::new(),
            replacement: vec![selector],
        },
    }
}

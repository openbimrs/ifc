//! Evidence-backed Nehirde correction ledger.

use crate::definition::{
    Applicability, CatalogEdition, PropertyDataType, PropertyKind, PropertyTemplate,
};

use super::{AdvisorySeverity, Patch, PatchOperation};

/// Ordered built-in patches for an exact source edition.
pub fn corrected_patches(edition: CatalogEdition) -> Vec<Patch> {
    match edition {
        CatalogEdition::Ifc4Add2Tc1 => vec![
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
        ],
        CatalogEdition::Ifc4x3Add2 => vec![Patch {
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
        }],
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

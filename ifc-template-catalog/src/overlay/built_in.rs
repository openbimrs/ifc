//! Evidence-backed Nehirde correction ledger.

use crate::definition::{Applicability, CatalogEdition};

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
        _ => Vec::new(),
    }
}

fn environmental_advisory(edition: CatalogEdition, id: &str, target: &str) -> Patch {
    Patch {
        id: id.into(),
        edition,
        target_template: target.into(),
        rationale: "Flag legacy scalar environmental data without changing official semantics".into(),
        evidence: "docs/adr/0010-versioned-psd-qto-catalog.md".into(),
        operation: PatchOperation::AddAdvisory {
            severity: AdvisorySeverity::Warning,
            message: "Legacy and underspecified for module-based EPD data; use an explicit EPD domain model"
                .into(),
        },
    }
}

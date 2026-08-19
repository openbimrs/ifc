//! Immutable patch application.

use std::collections::BTreeSet;

use crate::catalog::{Catalog, CatalogProfile};

use super::{Advisory, AppliedPatch, Patch, PatchError, PatchOperation};

impl Catalog {
    /// Apply an ordered patch list to a new immutable snapshot.
    pub fn with_patches(
        &self,
        profile: CatalogProfile,
        patches: &[Patch],
    ) -> Result<Self, PatchError> {
        if patches.is_empty() {
            return Err(PatchError::EmptyLedger);
        }
        if profile == CatalogProfile::Official
            || (profile == CatalogProfile::Corrected
                && (self.profile() != CatalogProfile::Official
                    || !self.applied_patches().is_empty()))
        {
            return Err(PatchError::InvalidProfileTransition {
                from: self.profile(),
                to: profile,
            });
        }
        let mut templates = self.clone().into_templates();
        let mut applied = self.applied_patches().to_vec();
        let mut advisories = self.advisories().to_vec();
        let mut ids: BTreeSet<String> = applied.iter().map(|patch| patch.id.clone()).collect();
        let mut replaced_applicability: BTreeSet<String> = applied
            .iter()
            .filter(|patch| {
                matches!(
                    &patch.operation,
                    PatchOperation::ReplaceApplicability { .. }
                )
            })
            .map(|patch| patch.target_template.clone())
            .collect();
        let mut added_applicability: BTreeSet<String> = applied
            .iter()
            .filter(|patch| matches!(&patch.operation, PatchOperation::AddApplicability(_)))
            .map(|patch| patch.target_template.clone())
            .collect();

        for patch in patches {
            if !ids.insert(patch.id.clone()) {
                return Err(PatchError::DuplicateId(patch.id.clone()));
            }
            if patch.edition != self.manifest().edition {
                return Err(PatchError::EditionMismatch {
                    patch_id: patch.id.clone(),
                    patch_edition: patch.edition,
                    catalog_edition: self.manifest().edition,
                });
            }
            let template = templates
                .iter_mut()
                .find(|template| template.name == patch.target_template)
                .ok_or_else(|| PatchError::UnknownTemplate {
                    patch_id: patch.id.clone(),
                    template: patch.target_template.clone(),
                })?;

            match &patch.operation {
                PatchOperation::AddApplicability(selector) => {
                    if replaced_applicability.contains(&template.name) {
                        return Err(PatchError::ConflictingApplicability {
                            template: template.name.clone(),
                        });
                    }
                    if template
                        .applicability
                        .iter()
                        .any(|existing| same_selector(existing, selector))
                    {
                        return Err(PatchError::AlreadyApplied {
                            patch_id: patch.id.clone(),
                            template: template.name.clone(),
                        });
                    }
                    template.applicability.push(selector.clone());
                    added_applicability.insert(template.name.clone());
                }
                PatchOperation::ReplaceApplicability {
                    expected,
                    replacement,
                } => {
                    if added_applicability.contains(&template.name)
                        || !replaced_applicability.insert(template.name.clone())
                    {
                        return Err(PatchError::ConflictingApplicability {
                            template: template.name.clone(),
                        });
                    }
                    if template.applicability != *expected {
                        return Err(PatchError::StaleTarget {
                            patch_id: patch.id.clone(),
                            template: template.name.clone(),
                        });
                    }
                    template.applicability = replacement.clone();
                }
                PatchOperation::AddAdvisory { severity, message } => {
                    advisories.push(Advisory {
                        patch_id: patch.id.clone(),
                        target_template: template.name.clone(),
                        severity: *severity,
                        message: message.clone(),
                        evidence: patch.evidence.clone(),
                    });
                }
            }
            applied.push(AppliedPatch {
                id: patch.id.clone(),
                target_template: patch.target_template.clone(),
                rationale: patch.rationale.clone(),
                evidence: patch.evidence.clone(),
                operation: patch.operation.clone(),
            });
        }

        let catalog = Catalog::try_new_with_profile(self.manifest().clone(), profile, templates)?;
        Ok(catalog.with_overlay_state(profile, applied, advisories))
    }
}

fn same_selector(
    left: &crate::definition::Applicability,
    right: &crate::definition::Applicability,
) -> bool {
    left.entity.eq_ignore_ascii_case(&right.entity)
        && match (&left.predefined_type, &right.predefined_type) {
            (None, None) => true,
            (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
            _ => false,
        }
}

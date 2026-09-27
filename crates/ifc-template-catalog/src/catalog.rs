//! Immutable catalog snapshots and indices.

use std::collections::BTreeMap;
use std::sync::Arc;

use thiserror::Error;

use crate::definition::{SetTemplate, SetTemplateKind, SourceManifest};
use crate::overlay::{Advisory, AppliedPatch};

/// Selected source-policy profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CatalogProfile {
    /// Unmodified upstream publication data, no correction overlay applied.
    Official,
    /// Official data with the built-in correction overlay's patches applied.
    Corrected,
    /// Official or corrected data with a caller-supplied overlay applied.
    Custom,
}

/// Immutable, cheaply cloneable template snapshot.
#[derive(Debug, Clone)]
pub struct Catalog(Arc<CatalogInner>);

#[derive(Debug, Clone)]
struct CatalogInner {
    manifest: SourceManifest,
    profile: CatalogProfile,
    templates: Vec<SetTemplate>,
    by_name: BTreeMap<String, usize>,
    applied_patches: Vec<AppliedPatch>,
    advisories: Vec<Advisory>,
}

impl Catalog {
    /// Build a snapshot for [`CatalogProfile::Official`] or
    /// [`CatalogProfile::Custom`]. Fails on duplicate/empty template names or
    /// a manifest set-count mismatch; use [`crate::overlay`] to build a
    /// [`CatalogProfile::Corrected`] snapshot instead, since this rejects that
    /// profile directly.
    pub fn try_new(
        manifest: SourceManifest,
        profile: CatalogProfile,
        templates: Vec<SetTemplate>,
    ) -> Result<Self, CatalogError> {
        if profile == CatalogProfile::Corrected {
            return Err(CatalogError::CorrectedProfileRequiresPatches);
        }
        Self::try_new_with_profile(manifest, profile, templates)
    }

    /// Build a snapshot for any profile, including [`CatalogProfile::Corrected`].
    /// Reserved for callers within this crate that have already produced an
    /// applied-patch ledger.
    pub(crate) fn try_new_with_profile(
        manifest: SourceManifest,
        profile: CatalogProfile,
        templates: Vec<SetTemplate>,
    ) -> Result<Self, CatalogError> {
        let mut by_name = BTreeMap::new();
        let mut property_sets = 0;
        let mut quantity_sets = 0;
        for (index, template) in templates.iter().enumerate() {
            if template.name.trim().is_empty() {
                return Err(CatalogError::EmptyTemplateName);
            }
            if by_name.insert(template.name.clone(), index).is_some() {
                return Err(CatalogError::DuplicateTemplate(template.name.clone()));
            }
            match template.kind {
                SetTemplateKind::Property { .. } => property_sets += 1,
                SetTemplateKind::Quantity { .. } => quantity_sets += 1,
            }
        }
        if manifest.property_set_count != property_sets
            || manifest.quantity_set_count != quantity_sets
        {
            return Err(CatalogError::ManifestCountMismatch {
                expected_property_sets: manifest.property_set_count,
                actual_property_sets: property_sets,
                expected_quantity_sets: manifest.quantity_set_count,
                actual_quantity_sets: quantity_sets,
            });
        }
        Ok(Self(Arc::new(CatalogInner {
            manifest,
            profile,
            templates,
            by_name,
            applied_patches: Vec::new(),
            advisories: Vec::new(),
        })))
    }

    /// Look up a set template by its exact `Name`.
    pub fn get(&self, name: &str) -> Option<&SetTemplate> {
        self.0
            .by_name
            .get(name)
            .map(|index| &self.0.templates[*index])
    }

    /// Iterate every set template in this snapshot, in load order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SetTemplate> {
        self.0.templates.iter()
    }

    /// Total number of set templates in this snapshot.
    pub fn len(&self) -> usize {
        self.0.templates.len()
    }

    /// True when the snapshot holds no set templates.
    pub fn is_empty(&self) -> bool {
        self.0.templates.is_empty()
    }

    /// Source-policy profile this snapshot was built with.
    pub fn profile(&self) -> CatalogProfile {
        self.0.profile
    }

    /// Source manifest this snapshot was built from.
    pub fn manifest(&self) -> &SourceManifest {
        &self.0.manifest
    }

    /// Overlay patches that were applied to reach this snapshot, empty
    /// unless `profile()` is [`CatalogProfile::Corrected`] or [`CatalogProfile::Custom`].
    pub fn applied_patches(&self) -> &[AppliedPatch] {
        &self.0.applied_patches
    }

    /// Advisories targeting the named template, empty if none apply.
    pub fn advisories_for(&self, template: &str) -> Vec<&Advisory> {
        self.0
            .advisories
            .iter()
            .filter(|advisory| advisory.target_template == template)
            .collect()
    }

    /// Every advisory recorded against this snapshot, regardless of target.
    pub(crate) fn advisories(&self) -> &[Advisory] {
        &self.0.advisories
    }

    /// Replace this snapshot's profile, applied-patch ledger, and advisories
    /// in place, cloning the shared inner state first if other handles exist.
    pub(crate) fn with_overlay_state(
        mut self,
        profile: CatalogProfile,
        applied_patches: Vec<AppliedPatch>,
        advisories: Vec<Advisory>,
    ) -> Self {
        let inner = Arc::make_mut(&mut self.0);
        inner.profile = profile;
        inner.applied_patches = applied_patches;
        inner.advisories = advisories;
        self
    }

    /// Unwrap the snapshot into its owned template list, cloning only if
    /// another handle to the shared inner state is still alive.
    pub(crate) fn into_templates(self) -> Vec<SetTemplate> {
        match Arc::try_unwrap(self.0) {
            Ok(inner) => inner.templates,
            Err(inner) => inner.templates.clone(),
        }
    }
}

/// Catalog construction failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CatalogError {
    /// [`Catalog::try_new`] was called with [`CatalogProfile::Corrected`];
    /// that profile can only be built by applying an overlay's patch ledger.
    #[error("the corrected profile requires an applied patch ledger")]
    CorrectedProfileRequiresPatches,
    /// A template's `name` was empty or whitespace-only.
    #[error("template name is empty")]
    EmptyTemplateName,
    /// Two templates in the input shared the same `name`.
    #[error("duplicate template `{0}`")]
    DuplicateTemplate(String),
    /// The manifest's declared property-set/quantity-set counts did not
    /// match the counts actually present in the template list.
    #[error(
        "source manifest counts differ: property sets {actual_property_sets}/{expected_property_sets}, quantity sets {actual_quantity_sets}/{expected_quantity_sets}"
    )]
    ManifestCountMismatch {
        /// Property-set count declared by the manifest.
        expected_property_sets: usize,
        /// Property-set count actually found in the template list.
        actual_property_sets: usize,
        /// Quantity-set count declared by the manifest.
        expected_quantity_sets: usize,
        /// Quantity-set count actually found in the template list.
        actual_quantity_sets: usize,
    },
}

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
    Official,
    Corrected,
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

    pub fn get(&self, name: &str) -> Option<&SetTemplate> {
        self.0
            .by_name
            .get(name)
            .map(|index| &self.0.templates[*index])
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SetTemplate> {
        self.0.templates.iter()
    }

    pub fn len(&self) -> usize {
        self.0.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.templates.is_empty()
    }

    pub fn profile(&self) -> CatalogProfile {
        self.0.profile
    }

    pub fn manifest(&self) -> &SourceManifest {
        &self.0.manifest
    }

    pub fn applied_patches(&self) -> &[AppliedPatch] {
        &self.0.applied_patches
    }

    pub fn advisories_for(&self, template: &str) -> Vec<&Advisory> {
        self.0
            .advisories
            .iter()
            .filter(|advisory| advisory.target_template == template)
            .collect()
    }

    pub(crate) fn advisories(&self) -> &[Advisory] {
        &self.0.advisories
    }

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
    #[error("the corrected profile requires an applied patch ledger")]
    CorrectedProfileRequiresPatches,
    #[error("template name is empty")]
    EmptyTemplateName,
    #[error("duplicate template `{0}`")]
    DuplicateTemplate(String),
    #[error(
        "source manifest counts differ: property sets {actual_property_sets}/{expected_property_sets}, quantity sets {actual_quantity_sets}/{expected_quantity_sets}"
    )]
    ManifestCountMismatch {
        expected_property_sets: usize,
        actual_property_sets: usize,
        expected_quantity_sets: usize,
        actual_quantity_sets: usize,
    },
}

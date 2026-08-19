//! Patch and advisory contracts.

use thiserror::Error;

use crate::definition::{Applicability, CatalogEdition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub id: String,
    pub edition: CatalogEdition,
    pub target_template: String,
    pub rationale: String,
    pub evidence: String,
    pub operation: PatchOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchOperation {
    AddApplicability(Applicability),
    ReplaceApplicability {
        expected: Vec<Applicability>,
        replacement: Vec<Applicability>,
    },
    AddAdvisory {
        severity: AdvisorySeverity,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdvisorySeverity {
    Information,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advisory {
    pub patch_id: String,
    pub target_template: String,
    pub severity: AdvisorySeverity,
    pub message: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedPatch {
    pub id: String,
    pub target_template: String,
    pub rationale: String,
    pub evidence: String,
    pub operation: PatchOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PatchError {
    #[error("a patch ledger must not be empty")]
    EmptyLedger,
    #[error("cannot apply patches from {from:?} into {to:?}")]
    InvalidProfileTransition {
        from: crate::catalog::CatalogProfile,
        to: crate::catalog::CatalogProfile,
    },
    #[error("duplicate patch id `{0}`")]
    DuplicateId(String),
    #[error("patch `{patch_id}` targets {patch_edition:?}, catalog is {catalog_edition:?}")]
    EditionMismatch {
        patch_id: String,
        patch_edition: CatalogEdition,
        catalog_edition: CatalogEdition,
    },
    #[error("patch `{patch_id}` targets unknown template `{template}`")]
    UnknownTemplate { patch_id: String, template: String },
    #[error("patch `{patch_id}` is already reflected in `{template}`")]
    AlreadyApplied { patch_id: String, template: String },
    #[error("patch `{patch_id}` expected different applicability on `{template}`")]
    StaleTarget { patch_id: String, template: String },
    #[error("patches conflict on `{template}` applicability")]
    ConflictingApplicability { template: String },
    #[error(transparent)]
    Catalog(#[from] crate::catalog::CatalogError),
}

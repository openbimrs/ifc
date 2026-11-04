//! Patch and advisory contracts.

use thiserror::Error;

use crate::definition::{
    Applicability, CatalogEdition, PropertyKind, PropertyTemplate, QuantityKind,
};

/// One ledger entry correcting or annotating an official set template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// Stable identifier for this patch, e.g. `NEH-IFC4-QTO-0001`.
    pub id: String,
    /// Source edition this patch targets; applying it to a catalog of a
    /// different edition fails.
    pub edition: CatalogEdition,
    /// `Name` of the set template this patch corrects or annotates.
    pub target_template: String,
    /// Why the correction is needed.
    pub rationale: String,
    /// Citation or reference supporting the correction.
    pub evidence: String,
    /// The concrete change this patch makes.
    pub operation: PatchOperation,
}

/// The kind of change one [`Patch`] makes to a set template.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchOperation {
    /// Add a new applicability selector, rejected if it duplicates an
    /// existing selector or if the target already has a
    /// [`ReplaceApplicability`](PatchOperation::ReplaceApplicability) applied.
    AddApplicability(Applicability),
    /// Replace the target's entire applicability list, rejected unless the
    /// current list equals `expected` exactly.
    ReplaceApplicability {
        /// Applicability list the target must currently have.
        expected: Vec<Applicability>,
        /// Applicability list to install in its place.
        replacement: Vec<Applicability>,
    },
    /// Append a property template the edition's documentation lists but its
    /// machine-readable PSD omits. Rejected when the target is a quantity set
    /// or already has a member of that name.
    AddProperty(PropertyTemplate),
    /// Replace the kind of one quantity of a quantity set, rejected unless
    /// the quantity currently has the `expected` kind. For a published kind
    /// that real files cannot match.
    ReplaceQuantityKind {
        /// `Name` of the quantity in the target set.
        member: String,
        /// Kind the quantity must currently have.
        expected: QuantityKind,
        /// Kind to install in its place.
        replacement: QuantityKind,
    },
    /// Replace the kind of one property of a property set, rejected unless
    /// the property currently has the `expected` kind. For a published kind
    /// that real files cannot match.
    ReplacePropertyKind {
        /// `Name` of the property in the target set.
        member: String,
        /// Kind the property must currently have.
        expected: PropertyKind,
        /// Kind to install in its place.
        replacement: PropertyKind,
    },
    /// Attach an [`Advisory`] to the target template without changing its data.
    AddAdvisory {
        /// How serious the advisory is.
        severity: AdvisorySeverity,
        /// Advisory text shown to callers.
        message: String,
    },
}

/// How serious an [`Advisory`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdvisorySeverity {
    /// Informational note; no action implied.
    Information,
    /// Caution: the template is usable but has a known caveat.
    Warning,
    /// The template should not be used as published without the caller's own remediation.
    Error,
}

/// A non-mutating annotation attached to a set template by a [`Patch`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Advisory {
    /// Id of the [`Patch`] that added this advisory.
    pub patch_id: String,
    /// `Name` of the set template this advisory targets.
    pub target_template: String,
    /// How serious the advisory is.
    pub severity: AdvisorySeverity,
    /// Advisory text shown to callers.
    pub message: String,
    /// Citation or reference supporting the advisory.
    pub evidence: String,
}

/// Record of one [`Patch`] that was successfully applied to a catalog snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AppliedPatch {
    /// Id of the applied patch.
    pub id: String,
    /// `Name` of the set template it was applied to.
    pub target_template: String,
    /// Why the correction was needed.
    pub rationale: String,
    /// Citation or reference supporting the correction.
    pub evidence: String,
    /// The change that was made.
    pub operation: PatchOperation,
}

/// Why applying a patch ledger to a catalog failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PatchError {
    /// The supplied patch list was empty.
    #[error("a patch ledger must not be empty")]
    EmptyLedger,
    /// The requested profile transition is not allowed, e.g. patching an
    /// `Official` catalog into `Official`, or patching into `Corrected` when
    /// the catalog is not a clean `Official` snapshot.
    #[error("cannot apply patches from {from:?} into {to:?}")]
    InvalidProfileTransition {
        /// The catalog's profile before this application.
        from: crate::catalog::CatalogProfile,
        /// The profile requested for the result.
        to: crate::catalog::CatalogProfile,
    },
    /// Two patches in the ledger shared the same `id`.
    #[error("duplicate patch id `{0}`")]
    DuplicateId(String),
    /// A patch's `edition` does not match the catalog's edition.
    #[error("patch `{patch_id}` targets {patch_edition:?}, catalog is {catalog_edition:?}")]
    EditionMismatch {
        /// Id of the offending patch.
        patch_id: String,
        /// Edition the patch declares.
        patch_edition: CatalogEdition,
        /// Edition of the catalog being patched.
        catalog_edition: CatalogEdition,
    },
    /// A patch's `target_template` does not match any template `Name` in the catalog.
    #[error("patch `{patch_id}` targets unknown template `{template}`")]
    UnknownTemplate {
        /// Id of the offending patch.
        patch_id: String,
        /// The unmatched target template name.
        template: String,
    },
    /// An [`PatchOperation::AddApplicability`] selector, or a
    /// [`PatchOperation::AddProperty`] member name, already exists on the target.
    #[error("patch `{patch_id}` is already reflected in `{template}`")]
    AlreadyApplied {
        /// Id of the offending patch.
        patch_id: String,
        /// The target template name.
        template: String,
    },
    /// A [`PatchOperation::ReplaceApplicability`]'s `expected` list, or a
    /// [`PatchOperation::ReplaceQuantityKind`] or
    /// [`PatchOperation::ReplacePropertyKind`]'s `expected` kind, did not
    /// match the template's current value.
    #[error("patch `{patch_id}` expected different applicability on `{template}`")]
    StaleTarget {
        /// Id of the offending patch.
        patch_id: String,
        /// The target template name.
        template: String,
    },
    /// A [`PatchOperation::AddProperty`] or
    /// [`PatchOperation::ReplacePropertyKind`] targets a quantity set template.
    #[error("patch `{patch_id}` changes a property of quantity set `{template}`")]
    NotAPropertySet {
        /// Id of the offending patch.
        patch_id: String,
        /// The target template name.
        template: String,
    },
    /// A [`PatchOperation::ReplaceQuantityKind`] targets a property set template.
    #[error("patch `{patch_id}` changes a quantity of property set `{template}`")]
    NotAQuantitySet {
        /// Id of the offending patch.
        patch_id: String,
        /// The target template name.
        template: String,
    },
    /// A [`PatchOperation::ReplaceQuantityKind`] or
    /// [`PatchOperation::ReplacePropertyKind`] names a member the target lacks.
    #[error("patch `{patch_id}` names unknown member `{member}` of `{template}`")]
    UnknownMember {
        /// Id of the offending patch.
        patch_id: String,
        /// The target template name.
        template: String,
        /// The unmatched member name.
        member: String,
    },
    /// Two patches in the ledger both attempt to change the same template's
    /// applicability in incompatible ways (e.g. add after replace).
    #[error("patches conflict on `{template}` applicability")]
    ConflictingApplicability {
        /// The target template name.
        template: String,
    },
    /// Building the resulting catalog snapshot failed.
    #[error(transparent)]
    Catalog(#[from] crate::catalog::CatalogError),
}

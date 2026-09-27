//! Declarative corrections, advisories, and conflict checks.

mod apply;
mod built_in;
mod patch;

pub use built_in::corrected_patches;

pub use patch::{Advisory, AdvisorySeverity, AppliedPatch, Patch, PatchError, PatchOperation};

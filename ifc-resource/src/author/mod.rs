//! Transaction-staged authoring for the bounded IFC4 resource slice.

mod draft;
mod editor;

pub use draft::{AllocationDraft, NestingDraft, ResourceDraft, ResourceTimeDraft};
pub use editor::ResourceEditor;

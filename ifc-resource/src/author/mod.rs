//! Transaction-staged authoring for the bounded IFC4 resource slice.

mod actor;
mod draft;
mod editor;

pub use actor::{ActorDraft, AssetDraft};
pub use draft::{AllocationDraft, NestingDraft, ResourceDraft, ResourceTimeDraft};
pub use editor::{AppliedValueDraft, ResourceEditor};

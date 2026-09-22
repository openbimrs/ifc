//! Transaction-staged authoring for the bounded IFC4 resource slice.

mod actor;
mod contact;
mod draft;
mod editor;

pub use actor::{ActorDraft, AssetDraft, InventoryDraft};
pub use contact::{ActorRoleDraft, PostalAddressDraft, TelecomAddressDraft, TelecomLists};
pub use draft::{AllocationDraft, NestingDraft, ResourceDraft, ResourceTimeDraft};
pub use editor::{AppliedValueDraft, ResourceEditor};

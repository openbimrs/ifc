//! `ifc-resource` -- bounded construction-resource semantics.
//!
//! The crate exposes schema-resolved borrowed projections for construction
//! resource occurrences, resource types, actor and inventory metadata,
//! authored `IfcResourceTime`, allocation relationships, and budgeted resource
//! composition, for IFC4 ADD2 TC1 and IFC4X3 ADD2. IFC2X3 TC1 is read
//! through its own table: shared concepts answer the shared accessors,
//! IFC2X3-only attributes have their own, and what IFC2X3 lacks is a typed
//! [`ResourceError::NotInSchema`] (see `ResourceView`). Selected authoring
//! APIs, IFC4 and IFC4X3 only, stage records through
//! `ifc_model::Transaction`.
//!
//! It does not schedule work, level resources, calculate costs or quantities,
//! interpret calendars, or claim generic actor/inventory schema conformance.

mod author;
mod error;
mod query;
mod resource;
mod usage;
mod view;

mod actor;
mod inventory;

pub use actor::{ActorRole, Organization, OrganizationRelationship, Person, PersonAndOrganization};
pub use author::{
    ActorDraft, ActorRoleDraft, AllocationDraft, AppliedValueDraft, AssetDraft, InventoryDraft,
    NestingDraft, PostalAddressDraft, ResourceDraft, ResourceEditor, ResourceTimeDraft,
    TelecomAddressDraft, TelecomLists,
};
pub use error::{ResourceError, ResourceResult};
pub use inventory::Inventory;
pub use query::ResourceAllocation;
pub use resource::{
    ConstructionResource, ConstructionResourceType, ResourceKind, ResourceTypeKind,
};
pub use usage::{
    ComplexQuantity, MeasureWithUnit, ResourceTime, SimpleQuantity, SimpleQuantityValue,
};
pub use view::ResourceView;

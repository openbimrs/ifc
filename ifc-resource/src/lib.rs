//! `ifc-resource` -- bounded IFC4 construction-resource semantics.
//!
//! The crate exposes schema-resolved borrowed projections for construction
//! resource occurrences, resource types, actor and inventory metadata,
//! authored `IfcResourceTime`, allocation relationships, and budgeted resource
//! composition. Selected authoring APIs stage records through
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
mod crew;
mod equipment;
mod inventory;
mod labour;
mod material;

pub use actor::{ActorRole, Organization, OrganizationRelationship, Person, PersonAndOrganization};
pub use author::{AllocationDraft, NestingDraft, ResourceDraft, ResourceEditor, ResourceTimeDraft};
pub use error::{ResourceError, ResourceResult};
pub use inventory::Inventory;
pub use query::ResourceAllocation;
pub use resource::{
    ConstructionResource, ConstructionResourceType, ResourceKind, ResourceTypeKind,
};
pub use usage::{ComplexQuantity, ResourceTime, SimpleQuantity, SimpleQuantityValue};
pub use view::ResourceView;

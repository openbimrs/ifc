//! Bounded approval-resource views and transaction-staged authoring.
//!
//! This crate owns `IfcApproval`, its resource-level relationships, and
//! `IfcRelAssociatesApproval`, read and written in the model's declared
//! release (IFC2X3, IFC4 or IFC4X3) by attribute name. It validates selected
//! WHERE/SELECT rules but does not implement workflow, authorization,
//! signatures, or policy decisions.

mod authoring;
mod error;
mod projection;
mod release;
mod view;

pub use authoring::{
    associate_approval, associate_approval_with_owner_history, create_approval, relate_approvals,
    relate_resource_approval, ApprovalAssociationDraft, ApprovalDraft, ApprovalRelationshipDraft,
    DateTimeInput, ResourceApprovalDraft,
};
pub use error::{ApprovalError, ApprovalResult};
/// The IFC release a projection reads against (re-exported from
/// `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use projection::{
    Approval, ApprovalAssignment, ApprovalRelationship, ResourceApprovalRelationship,
};
pub use view::ApprovalView;

//! Bounded IFC4 approval-resource views and transaction-staged authoring.
//!
//! This crate owns `IfcApproval`, its resource-level relationships, and
//! `IfcRelAssociatesApproval`. It validates selected IFC4 WHERE/SELECT rules but
//! does not implement workflow, authorization, signatures, or policy decisions.
#![deny(missing_docs)]

mod authoring;
mod error;
mod projection;
mod view;

pub use authoring::{
    associate_approval, create_approval, relate_approvals, relate_resource_approval,
    ApprovalAssociationDraft, ApprovalDraft, ApprovalRelationshipDraft, ResourceApprovalDraft,
};
pub use error::{ApprovalError, ApprovalResult};
pub use projection::{
    Approval, ApprovalAssignment, ApprovalRelationship, ResourceApprovalRelationship,
};
pub use view::ApprovalView;

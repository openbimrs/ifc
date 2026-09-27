//! Format-neutral template application and compliance validation.

mod apply;
mod contract;
mod validate;

pub use apply::{apply_template, TemplateSink};
pub use contract::{
    MemberForm, ObservedMember, ObservedSet, UnexpectedMemberPolicy, ValidationCode,
    ValidationIssue, ValidationPolicy, ValidationReport, ValidationSeverity,
};
pub use validate::validate;

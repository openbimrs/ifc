//! Typed failures for bounded IFC4 approval semantics.

use ifc_model::EntityId;
use thiserror::Error;

/// Approval projection or authoring failure.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ApprovalError {
    /// Entity kind differs from the requested projection.
    #[error("expected {expected}, found {actual}")]
    WrongEntityType {
        /// Expected IFC entity.
        expected: &'static str,
        /// Actual IFC entity.
        actual: String,
    },
    /// Required positional attribute is absent or null.
    #[error("{entity} {id} is missing {attribute}")]
    MissingAttribute {
        /// Entity kind.
        entity: &'static str,
        /// Entity identifier.
        id: EntityId,
        /// Attribute name.
        attribute: &'static str,
    },
    /// Attribute has the wrong value shape or cardinality.
    #[error("{entity} {id}.{attribute} is invalid: {value}")]
    InvalidValue {
        /// Entity kind.
        entity: &'static str,
        /// Entity identifier.
        id: EntityId,
        /// Attribute name.
        attribute: &'static str,
        /// Diagnostic value.
        value: String,
    },
    /// Requested entity is absent.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// Missing identifier.
        id: EntityId,
    },
    /// Authored or projected reference does not resolve.
    #[error("{entity} {id}.{attribute} reference {target} does not resolve")]
    DanglingReference {
        /// Relationship or record kind.
        entity: &'static str,
        /// Owning entity identifier.
        id: EntityId,
        /// Attribute name.
        attribute: &'static str,
        /// Missing target.
        target: EntityId,
    },
    /// Resolved reference is outside the declared IFC SELECT/type.
    #[error("{entity} {id}.{attribute} target {target} has {actual}, expected {expected}")]
    ReferenceType {
        /// Relationship or record kind.
        entity: &'static str,
        /// Owning entity identifier.
        id: EntityId,
        /// Attribute name.
        attribute: &'static str,
        /// Referenced target.
        target: EntityId,
        /// Expected entity or SELECT.
        expected: &'static str,
        /// Actual entity kind.
        actual: String,
    },
    /// Declared WHERE-style rule failed.
    #[error("{entity} {id} violates {rule}: {detail}")]
    Semantic {
        /// Entity kind.
        entity: &'static str,
        /// Entity identifier.
        id: EntityId,
        /// Bounded rule name.
        rule: &'static str,
        /// Human-readable detail.
        detail: String,
    },
    /// Draft value is invalid before staging.
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        /// Entity kind.
        entity: &'static str,
        /// Attribute or rule.
        attribute: &'static str,
        /// Rejected value.
        value: String,
    },
    /// Draft reference is outside the required entity/SELECT.
    #[error("authoring target {target} has {actual}, expected {expected}")]
    AuthoringReferenceType {
        /// Referenced target.
        target: EntityId,
        /// Expected entity or SELECT.
        expected: &'static str,
        /// Actual entity kind.
        actual: String,
    },
}

/// Result alias for approval operations.
pub type ApprovalResult<T> = Result<T, ApprovalError>;

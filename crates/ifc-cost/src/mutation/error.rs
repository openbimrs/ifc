//! Typed refusal reasons for cost authoring.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;
use thiserror::Error;

/// Why a bounded IFC4 cost draft was refused before staging.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CostAuthoringError {
    /// A scalar, enum-dependent field, aggregate, or identifier was invalid.
    #[error("invalid {entity}.{attribute}: {reason}")]
    InvalidValue {
        /// IFC entity type being authored.
        entity: &'static str,
        /// IFC attribute being validated.
        attribute: &'static str,
        /// Stable human-readable refusal reason.
        reason: String,
    },
    /// A draft referenced an entity absent from the projected transaction state.
    #[error("{entity}.{attribute} references missing {target}")]
    MissingReference {
        /// IFC entity type being authored.
        entity: &'static str,
        /// IFC reference attribute being validated.
        attribute: &'static str,
        /// Missing entity identifier.
        target: EntityId,
    },
    /// A referenced entity had the wrong exact type for this bounded contract.
    #[error("{entity}.{attribute} references {target} of type {actual}; expected {expected}")]
    WrongReferenceType {
        /// IFC entity type being authored.
        entity: &'static str,
        /// IFC reference attribute being validated.
        attribute: &'static str,
        /// Referenced entity identifier.
        target: EntityId,
        /// Actual projected entity type.
        actual: String,
        /// Required exact IFC4 entity type.
        expected: &'static str,
    },
    /// A cost item was already nested under another parent.
    #[error("cost item {child} already has parent {existing_parent}")]
    MultipleParents {
        /// Child item being attached.
        child: EntityId,
        /// Existing parent in the projected transaction state.
        existing_parent: EntityId,
    },
    /// A proposed cost-item edge would create a self-reference or cycle.
    #[error("cost item nesting would create a cycle through {item}")]
    NestingCycle {
        /// Item at which cycle validation refused the draft.
        item: EntityId,
    },
    /// The model's header declares several schemas; release-bound
    /// authoring binds to exactly one.
    #[error("the header declares {schemas} schemas; authoring binds to exactly one")]
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The model's header declares one schema with no bundled table, so no
    /// layout can be trusted.
    #[error("the header declares {schema}, which has no bundled table")]
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// The model's release does not declare this entity, such as
    /// `IfcQuantityNumber` (IFC4X3 only) in an IFC2X3 or IFC4 model.
    #[error("{entity} is not an entity of {schema:?}")]
    EntityNotInSchema {
        /// IFC entity type being authored.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A draft supplied a value for an attribute the model's release does
    /// not declare, such as a `Formula` for an IFC2X3 quantity. It is
    /// refused rather than dropped.
    #[error("cannot author {entity}.{attribute}: not defined by {schema:?}")]
    AuthoringNotInSchema {
        /// IFC entity type being authored.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
}

/// Result returned by bounded cost authoring helpers.
pub type CostAuthoringResult<T> = Result<T, CostAuthoringError>;

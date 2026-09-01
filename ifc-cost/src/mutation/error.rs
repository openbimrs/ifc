//! Typed refusal reasons for cost authoring.

use ifc_model::EntityId;
use thiserror::Error;

/// Why a bounded IFC4 cost draft was refused before staging.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
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
}

/// Result returned by bounded cost authoring helpers.
pub type CostAuthoringResult<T> = Result<T, CostAuthoringError>;

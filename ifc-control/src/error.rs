//! Typed failures for bounded IFC control semantics.

use ifc_model::EntityId;
use thiserror::Error;

/// Control projection or authoring failure.
///
/// Only the variants this crate can actually raise are declared. A
/// variant that no code path constructs is a promise the crate does
/// not keep, and callers write dead match arms for it.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ControlError {
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
    /// Requested entity is absent.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// Missing identifier.
        id: EntityId,
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
    /// The schema in use does not declare this entity.
    ///
    /// Authoring an entity the schema omits is a caller error, not a
    /// silently-skipped attribute.
    #[error("{schema} does not declare {entity}")]
    UnsupportedEntity {
        /// Schema name.
        schema: String,
        /// Entity that is not declared.
        entity: &'static str,
    },
}

/// Result alias for control operations.
pub type ControlResult<T> = Result<T, ControlError>;

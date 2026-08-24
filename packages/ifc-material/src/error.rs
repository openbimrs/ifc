//! Typed failures while interpreting IFC material-resource entities.

use ifc_model::EntityId;
use thiserror::Error;

/// A malformed, ambiguous, or unresolved material projection.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum MaterialError {
    #[error("expected {expected}, found {actual}")]
    WrongEntityType {
        expected: &'static str,
        actual: String,
    },
    #[error("{entity} {id} is missing required attribute {attribute}")]
    MissingAttribute {
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    },
    #[error("{entity} {id} has invalid {attribute}: {value}")]
    InvalidValue {
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
        value: String,
    },
    #[error("entity {id} does not exist")]
    UnknownEntity { id: EntityId },
    #[error("reference {target} from {source_id} does not resolve")]
    DanglingReference {
        source_id: EntityId,
        target: EntityId,
    },
    #[error("reference {target} from {source_id} has type {actual}, expected {expected}")]
    ReferenceType {
        source_id: EntityId,
        target: EntityId,
        expected: &'static str,
        actual: String,
    },
    #[error("object {object} has {count} direct material assignments")]
    AmbiguousAssignment { object: EntityId, count: usize },
    #[error("object {object} has {count} assigned IFC types")]
    AmbiguousType { object: EntityId, count: usize },
}

pub type MaterialResult<T> = Result<T, MaterialError>;

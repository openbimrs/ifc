//! Typed failures while projecting or authoring presentation data.

use ifc_model::EntityId;
use thiserror::Error;

/// A malformed, unresolved, or unsupported presentation contract.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum StyleError {
    #[error("entity {id} does not exist")]
    UnknownEntity { id: EntityId },
    #[error("expected {expected}, found {actual} at {id}")]
    WrongEntityType {
        id: EntityId,
        expected: &'static str,
        actual: String,
    },
    #[error("schema {schema} does not declare {entity}")]
    UnsupportedEntity {
        schema: String,
        entity: &'static str,
    },
    #[error("{entity} {id} is missing required attribute {attribute}")]
    MissingAttribute {
        entity: String,
        id: EntityId,
        attribute: &'static str,
    },
    #[error("{entity} {id} has invalid {attribute}: {value}")]
    InvalidValue {
        entity: String,
        id: EntityId,
        attribute: &'static str,
        value: String,
    },
    #[error("entity {entity} in schema {schema} has no attribute {attribute}")]
    UnsupportedAttribute {
        schema: String,
        entity: &'static str,
        attribute: &'static str,
    },
    #[error("reference {target} from {source_id} does not resolve")]
    DanglingReference {
        source_id: EntityId,
        target: EntityId,
    },
    #[error("reference {target} has type {actual}, expected {expected}")]
    ReferenceType {
        target: EntityId,
        expected: &'static str,
        actual: String,
    },
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        entity: &'static str,
        attribute: &'static str,
        value: String,
    },
    #[error("{entity} {id}.{attribute} is outside [{minimum}, {maximum}]: {value}")]
    OutOfRange {
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
        value: f64,
        minimum: f64,
        maximum: f64,
    },
    #[error("representation item {item} has {count} direct styled-item assignments")]
    AmbiguousStyleAssignment { item: EntityId, count: usize },
}

pub type StyleResult<T> = Result<T, StyleError>;

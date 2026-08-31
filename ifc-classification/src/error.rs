//! Typed failures while interpreting or authoring IFC external references.

use ifc_model::EntityId;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ClassificationError {
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
    #[error("{entity} {id}.{attribute} reference {target} does not resolve")]
    DanglingReference {
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
        target: EntityId,
    },
    #[error("{entity} {id}.{attribute} reference {target} has type {actual}, expected {expected}")]
    ReferenceType {
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
        target: EntityId,
        expected: &'static str,
        actual: String,
    },
    #[error("classification hierarchy contains a cycle: {path:?}")]
    Cycle { path: Vec<EntityId> },
    #[error("classification hierarchy exceeded max_depth={max_depth} or max_nodes={max_nodes}")]
    BudgetExceeded { max_depth: usize, max_nodes: usize },
    #[error("object {object} has {count} assigned IFC types")]
    AmbiguousType { object: EntityId, count: usize },
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        entity: &'static str,
        attribute: &'static str,
        value: String,
    },
    #[error("authoring reference {target} has type {actual}, expected {expected}")]
    AuthoringReferenceType {
        target: EntityId,
        expected: &'static str,
        actual: String,
    },
}

pub type ClassificationResult<T> = Result<T, ClassificationError>;

//! Typed failures while interpreting or authoring IFC external references.

use ifc_model::EntityId;
use thiserror::Error;

/// Failure decoding, cross-checking, or authoring classification/document/library records.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ClassificationError {
    /// Entity did not have the expected IFC type.
    #[error("expected {expected}, found {actual}")]
    WrongEntityType {
        /// IFC type name expected at this position.
        expected: &'static str,
        /// IFC type name actually found.
        actual: String,
    },
    /// A required attribute was unset (`$` or `*`) or absent.
    #[error("{entity} {id} is missing required attribute {attribute}")]
    MissingAttribute {
        /// IFC entity type of the offending instance.
        entity: &'static str,
        /// Id of the offending instance.
        id: EntityId,
        /// Name of the missing attribute.
        attribute: &'static str,
    },
    /// An attribute was present but failed to decode as its expected shape.
    #[error("{entity} {id} has invalid {attribute}: {value}")]
    InvalidValue {
        /// IFC entity type of the offending instance.
        entity: &'static str,
        /// Id of the offending instance.
        id: EntityId,
        /// Name of the invalid attribute.
        attribute: &'static str,
        /// Debug rendering of the offending value.
        value: String,
    },
    /// A referenced entity id does not exist in the model.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// Id that could not be resolved.
        id: EntityId,
    },
    /// An attribute reference points at an id that is not present in the model.
    #[error("{entity} {id}.{attribute} reference {target} does not resolve")]
    DanglingReference {
        /// IFC entity type holding the dangling reference.
        entity: &'static str,
        /// Id of the entity holding the dangling reference.
        id: EntityId,
        /// Name of the attribute holding the dangling reference.
        attribute: &'static str,
        /// Id that could not be resolved.
        target: EntityId,
    },
    /// An attribute reference resolves but the target entity has the wrong type for its select.
    #[error("{entity} {id}.{attribute} reference {target} has type {actual}, expected {expected}")]
    ReferenceType {
        /// IFC entity type holding the mistyped reference.
        entity: &'static str,
        /// Id of the entity holding the mistyped reference.
        id: EntityId,
        /// Name of the attribute holding the mistyped reference.
        attribute: &'static str,
        /// Id of the mistyped target.
        target: EntityId,
        /// Select or type name the target was expected to satisfy.
        expected: &'static str,
        /// IFC type name the target actually has.
        actual: String,
    },
    /// Following `ReferencedSource` links revisited an already-seen entity.
    #[error("classification hierarchy contains a cycle: {path:?}")]
    Cycle {
        /// Entity ids on the cycle, in traversal order.
        path: Vec<EntityId>,
    },
    /// Hierarchy traversal exceeded the caller-supplied node or depth budget.
    #[error("classification hierarchy exceeded max_depth={max_depth} or max_nodes={max_nodes}")]
    BudgetExceeded {
        /// Maximum edge depth allowed.
        max_depth: usize,
        /// Maximum number of nodes allowed.
        max_nodes: usize,
    },
    /// An occurrence object is related to more than one `IfcRelDefinesByType` type.
    #[error("object {object} has {count} assigned IFC types")]
    AmbiguousType {
        /// The occurrence object with the ambiguous type assignment.
        object: EntityId,
        /// Number of types the object was found related to.
        count: usize,
    },
    /// A draft value failed schema validation before staging.
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        /// IFC entity type being authored.
        entity: &'static str,
        /// Name of the offending attribute.
        attribute: &'static str,
        /// Description of why the value is invalid.
        value: String,
    },
    /// A draft reference resolves to a staged or committed entity of the wrong type.
    #[error("authoring reference {target} has type {actual}, expected {expected}")]
    AuthoringReferenceType {
        /// Id of the mistyped reference target.
        target: EntityId,
        /// Select or type name the target was expected to satisfy.
        expected: &'static str,
        /// IFC type name the target actually has.
        actual: String,
    },
}

/// Result of a classification/document/library query or authoring operation.
pub type ClassificationResult<T> = Result<T, ClassificationError>;

//! Typed failures while interpreting IFC material-resource entities.

use ifc_model::EntityId;
use thiserror::Error;

/// A malformed, ambiguous, or unresolved material projection.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum MaterialError {
    /// A projection expected one entity type but the model holds another.
    #[error("expected {expected}, found {actual}")]
    WrongEntityType {
        /// The IFC entity type name that was expected.
        expected: &'static str,
        /// The IFC entity type name actually found on the record.
        actual: String,
    },
    /// A required attribute slot on the entity is absent (unset `$`).
    #[error("{entity} {id} is missing required attribute {attribute}")]
    MissingAttribute {
        /// The IFC entity type name.
        entity: &'static str,
        /// The id of the entity missing the attribute.
        id: EntityId,
        /// The name of the missing attribute.
        attribute: &'static str,
    },
    /// An attribute is present but its value violates a WHERE rule or type
    /// constraint.
    #[error("{entity} {id} has invalid {attribute}: {value}")]
    InvalidValue {
        /// The IFC entity type name.
        entity: &'static str,
        /// The id of the entity with the invalid attribute.
        id: EntityId,
        /// The name of the invalid attribute.
        attribute: &'static str,
        /// A rendering of the offending value.
        value: String,
    },
    /// A value supplied to an authoring function cannot be written to the
    /// named entity attribute.
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        /// The IFC entity type name being authored.
        entity: &'static str,
        /// The name of the attribute that rejected the value.
        attribute: &'static str,
        /// A rendering of the rejected value.
        value: String,
    },
    /// An authoring-time reference points at an entity of the wrong type.
    #[error("authoring reference {target} has type {actual}, expected {expected}")]
    AuthoringReferenceType {
        /// The id of the referenced entity.
        target: EntityId,
        /// The IFC entity type name that was expected.
        expected: &'static str,
        /// The IFC entity type name actually found at `target`.
        actual: String,
    },
    /// A referenced entity id does not exist in the model.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// The id that could not be resolved.
        id: EntityId,
    },
    /// A reference field points at an id absent from the model.
    #[error("reference {target} from {source_id} does not resolve")]
    DanglingReference {
        /// The id of the entity holding the reference.
        source_id: EntityId,
        /// The unresolved target id.
        target: EntityId,
    },
    /// A reference resolves to an entity, but of the wrong type.
    #[error("reference {target} from {source_id} has type {actual}, expected {expected}")]
    ReferenceType {
        /// The id of the entity holding the reference.
        source_id: EntityId,
        /// The id of the referenced entity.
        target: EntityId,
        /// The IFC entity type name that was expected.
        expected: &'static str,
        /// The IFC entity type name actually found at `target`.
        actual: String,
    },
    /// The object is directly related to more than one `IfcMaterial`
    /// via `IfcRelAssociatesMaterial`, so the single active material cannot
    /// be determined.
    #[error("object {object} has {count} direct material assignments")]
    AmbiguousAssignment {
        /// The id of the object with conflicting assignments.
        object: EntityId,
        /// The number of direct material assignments found.
        count: usize,
    },
    /// The object is typed by more than one `IfcTypeObject`, so its
    /// inherited material cannot be determined unambiguously.
    #[error("object {object} has {count} assigned IFC types")]
    AmbiguousType {
        /// The id of the object with conflicting type relationships.
        object: EntityId,
        /// The number of `IfcTypeObject` relationships found.
        count: usize,
    },
}

/// Convenience alias for results of material-projection operations.
pub type MaterialResult<T> = Result<T, MaterialError>;

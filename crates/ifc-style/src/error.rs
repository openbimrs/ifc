//! Typed failures while projecting or authoring presentation data.

use ifc_model::EntityId;
use thiserror::Error;

/// A malformed, unresolved, or unsupported presentation contract.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum StyleError {
    /// An entity id was looked up but the model does not contain it.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// The id that resolved to nothing.
        id: EntityId,
    },
    /// An entity was read as one IFC type but is actually another.
    #[error("expected {expected}, found {actual} at {id}")]
    WrongEntityType {
        /// The entity that was read.
        id: EntityId,
        /// The IFC type the caller expected.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// The active schema does not declare the requested entity type.
    #[error("schema {schema} does not declare {entity}")]
    UnsupportedEntity {
        /// The schema's name.
        schema: String,
        /// The entity type name that is undeclared.
        entity: &'static str,
    },
    /// A mandatory attribute was absent, so the value cannot be inferred.
    #[error("{entity} {id} is missing required attribute {attribute}")]
    MissingAttribute {
        /// The entity's IFC type.
        entity: String,
        /// The entity that was read.
        id: EntityId,
        /// Schema name of the attribute.
        attribute: &'static str,
    },
    /// An attribute was present but held the wrong kind of value.
    #[error("{entity} {id} has invalid {attribute}: {value}")]
    InvalidValue {
        /// The entity's IFC type.
        entity: String,
        /// The entity that was read.
        id: EntityId,
        /// Schema name of the attribute.
        attribute: &'static str,
        /// Debug-formatted offending value.
        value: String,
    },
    /// The active schema declares the entity but not the requested attribute slot.
    #[error("entity {entity} in schema {schema} has no attribute {attribute}")]
    UnsupportedAttribute {
        /// The schema's name.
        schema: String,
        /// The entity type name.
        entity: &'static str,
        /// The attribute name that is undeclared.
        attribute: &'static str,
    },
    /// A reference attribute pointed at an entity id the model does not contain.
    #[error("reference {target} from {source_id} does not resolve")]
    DanglingReference {
        /// The entity holding the dangling reference.
        source_id: EntityId,
        /// The id that resolved to nothing.
        target: EntityId,
    },
    /// A reference resolved, but the target is not the IFC type (or select
    /// member) the referencing slot requires.
    #[error("reference {target} has type {actual}, expected {expected}")]
    ReferenceType {
        /// The entity that was read.
        target: EntityId,
        /// The IFC type (or select) the slot requires.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// An authoring helper was given a value it cannot encode for the target attribute.
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        /// The IFC entity type being authored.
        entity: &'static str,
        /// The attribute being set.
        attribute: &'static str,
        /// Debug-formatted offending value.
        value: String,
    },
    /// A normalized-ratio or bounded measure fell outside its required range.
    #[error("{entity} {id}.{attribute} is outside [{minimum}, {maximum}]: {value}")]
    OutOfRange {
        /// The measure's IFC type, e.g. `"IfcNormalisedRatioMeasure"`.
        entity: &'static str,
        /// The entity that was read.
        id: EntityId,
        /// Schema name of the attribute.
        attribute: &'static str,
        /// The out-of-range value.
        value: f64,
        /// The inclusive lower bound.
        minimum: f64,
        /// The inclusive upper bound.
        maximum: f64,
    },
    /// More than one `IfcStyledItem` directly assigned styles to the same
    /// representation item; cascade resolution refuses to guess a winner.
    #[error("representation item {item} has {count} direct styled-item assignments")]
    AmbiguousStyleAssignment {
        /// The representation item with conflicting direct assignments.
        item: EntityId,
        /// The number of conflicting `IfcStyledItem`s found.
        count: usize,
    },
}

/// The result type returned by this crate's projection and authoring operations.
pub type StyleResult<T> = Result<T, StyleError>;

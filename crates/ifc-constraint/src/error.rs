//! Typed failures for bounded IFC4 constraint semantics.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;
use thiserror::Error;

/// Constraint projection or authoring failure.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ConstraintError {
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
    /// The model's header declares several schemas; authoring binds to
    /// exactly one release.
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
    /// The model's release requires an attribute the authoring call leaves
    /// unset, such as the IFC2X3 `IfcRoot.OwnerHistory` or the IFC2X3
    /// `IfcRelAssociatesConstraint.Intent`.
    #[error("cannot author {entity}: {schema:?} requires {attribute}")]
    AuthoringRequired {
        /// The entity type being authored.
        entity: &'static str,
        /// The required attribute, as the release names it.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
}

/// Result alias for constraint operations.
pub type ConstraintResult<T> = Result<T, ConstraintError>;

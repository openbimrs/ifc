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
    /// The model's header declares one schema this crate is not verified
    /// against (anything but IFC2X3, IFC4 and IFC4X3), so no layout can be
    /// trusted. Never read or written as another release.
    #[error("the header declares {schema}, which this crate is not verified against")]
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// An accessor asked for an attribute the model's release does not
    /// declare, such as `ReferencePath` on an IFC2X3 `IfcMetric`. Never read
    /// from the slot another release gives it (#212).
    #[error("{entity} {id}.{attribute} is not defined by {schema:?}")]
    NotInSchema {
        /// Entity kind.
        entity: &'static str,
        /// Entity identifier.
        id: EntityId,
        /// The attribute.
        attribute: &'static str,
        /// The release the model is read against.
        schema: SchemaVersion,
    },
    /// A text accessor met an attribute the release types as an entity
    /// record, such as the IFC2X3 `IfcConstraint.CreationTime`, an
    /// `IfcDateTimeSelect`. The value is valid; read it through `target`.
    #[error("{entity} {id}.{attribute} is the record {target}, not text")]
    StructuredValue {
        /// Entity kind.
        entity: &'static str,
        /// Entity identifier.
        id: EntityId,
        /// The attribute.
        attribute: &'static str,
        /// The entity record holding the value.
        target: EntityId,
    },
    /// The model's release does not declare this entity, such as
    /// `IfcResourceConstraintRelationship` or `IfcReference` in IFC2X3.
    #[error("{entity} is not an instantiable entity of {schema:?}")]
    EntityNotInSchema {
        /// The entity type.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A draft supplied a value for an attribute the model's release does
    /// not declare, such as a `ReferencePath` for an IFC2X3 `IfcMetric`. It
    /// is refused rather than dropped.
    #[error("cannot author {entity}.{attribute}: not defined by {schema:?}")]
    AuthoringNotInSchema {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A draft supplied a value in a form the release's declaration cannot
    /// hold, such as text where IFC2X3 declares an `IfcDateTimeSelect`
    /// record, a token its enumeration lacks, or several benchmark values
    /// where IFC2X3 declares one `IfcMetric`.
    #[error("cannot author {entity}.{attribute}: {schema:?} declares {declared}")]
    AuthoringValueType {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The type the release declares.
        declared: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The model's release requires an attribute the authoring call leaves
    /// unset, such as the IFC2X3 `IfcRoot.OwnerHistory`, the IFC2X3
    /// `IfcRelAssociatesConstraint.Intent` or the IFC2X3 `IfcMetric.DataValue`.
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

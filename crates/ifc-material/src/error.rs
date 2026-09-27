//! Typed failures while interpreting IFC material-resource entities.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;
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
    /// The header declares several schemas, so no single release can be
    /// bound to read or write the model against.
    #[error("the header declares {schemas} schemas; material reads bind to exactly one")]
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The header declares one schema this crate has no bundled table for,
    /// so no slot position can be trusted.
    #[error("the header declares {schema}, which has no bundled schema table")]
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// The release the model is read against does not declare this
    /// attribute, for example `Category` on an IFC2X3 `IfcMaterial` or
    /// `Name` on an IFC2X3 `IfcMaterialLayer`. This is never reported as an
    /// unset (`None`) value, and no other slot is read in its place.
    #[error("{entity} {id}.{attribute} is not defined by {schema:?}")]
    NotInSchema {
        /// IFC entity type of the instance.
        entity: &'static str,
        /// Id of the instance.
        id: EntityId,
        /// Attribute name, as this crate's accessor names it (IFC4).
        attribute: &'static str,
        /// The release the model is read against.
        schema: SchemaVersion,
    },
    /// The release the model is bound to has no instantiable entity of this
    /// type, for example an `IfcMaterialConstituentSet` or
    /// `IfcMaterialProfileSet` in an IFC2X3 model, or the abstract IFC2X3
    /// `IfcMaterialProperties`. Such a record is refused, not decoded with
    /// another release's layout.
    #[error("{entity} is not an instantiable entity of {schema:?}")]
    EntityNotInSchema {
        /// The IFC entity type name.
        entity: &'static str,
        /// The record, when one is being read; `None` when authoring.
        id: Option<EntityId>,
        /// The release the model is bound to.
        schema: SchemaVersion,
    },
    /// An authoring call supplied a value for an attribute the model's
    /// release does not declare, such as a layer `Name` for an IFC2X3
    /// model. The value is refused rather than silently dropped.
    #[error("cannot author {entity}.{attribute}: not defined by {schema:?}")]
    AuthoringNotInSchema {
        /// The IFC entity type name being authored.
        entity: &'static str,
        /// The attribute, as this crate names it (IFC4).
        attribute: &'static str,
        /// The release the model is bound to.
        schema: SchemaVersion,
    },
    /// The model's release requires an attribute the authoring call leaves
    /// unset, such as the IFC2X3 `IfcRoot.OwnerHistory`.
    #[error("cannot author {entity}: {schema:?} requires {attribute}")]
    AuthoringRequired {
        /// The IFC entity type name being authored.
        entity: &'static str,
        /// The required attribute, as the release names it.
        attribute: &'static str,
        /// The release the model is bound to.
        schema: SchemaVersion,
    },
}

/// Convenience alias for results of material-projection operations.
pub type MaterialResult<T> = Result<T, MaterialError>;

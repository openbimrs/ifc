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
    /// Draft value is invalid before staging, including an `OwnerHistory`
    /// that is not an `IfcOwnerHistory`.
    #[error("cannot author {entity}.{attribute}: {value}")]
    AuthoringInvalid {
        /// Entity kind.
        entity: &'static str,
        /// Attribute or rule.
        attribute: &'static str,
        /// Rejected value.
        value: String,
    },
    /// The relating control of an assignment is not one this crate owns.
    ///
    /// Cost schedules, cost items and work controls are `IfcControl`s
    /// too; their own crates write assignments to them.
    #[error("{id} is {actual}, not a control ifc-control owns")]
    ForeignControl {
        /// The offered relating control.
        id: EntityId,
        /// Its type name.
        actual: String,
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
    /// The model's header declares several schemas; release-bound
    /// authoring binds to exactly one.
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
    /// A draft supplied a value for an attribute the release does not
    /// declare, such as a `PredefinedType` for an IFC2X3 `IfcPermit`. It is
    /// refused rather than dropped.
    #[error("cannot author {entity}.{attribute}: not declared by {schema}")]
    AuthoringNotInSchema {
        /// Entity kind.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// Schema name.
        schema: String,
    },
    /// A draft supplied a value the release's declaration of the attribute
    /// cannot hold.
    #[error("cannot author {entity}.{attribute}: {schema} declares it {declared}")]
    AuthoringValueType {
        /// Entity kind.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// The type the release declares.
        declared: String,
        /// Schema name.
        schema: String,
    },
    /// The release requires an attribute the call leaves unset, such as the
    /// IFC2X3 `IfcRoot.OwnerHistory` (#198, #202) or `IfcPermit.PermitID`.
    /// It is refused rather than written as `$`; the `*_with_owner_history`
    /// writers take the `IfcOwnerHistory` IFC2X3 needs.
    #[error("cannot author {entity}: {schema} requires {attribute}")]
    AuthoringRequired {
        /// Entity kind.
        entity: &'static str,
        /// The attribute, by the release's own name.
        attribute: String,
        /// Schema name.
        schema: String,
    },
}

/// Result alias for control operations.
pub type ControlResult<T> = Result<T, ControlError>;

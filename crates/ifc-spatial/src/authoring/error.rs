//! Why a spatial record was refused.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

/// Why a spatial record was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpatialAuthoringError {
    /// A value the schema constrains was not acceptable.
    Invalid {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute at fault.
        attribute: &'static str,
        /// What was supplied.
        value: String,
    },
    /// The model's header declares several schemas; release-bound
    /// authoring binds to exactly one.
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The model's header declares one schema with no bundled table, so no
    /// layout can be trusted.
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// The model's release does not declare this entity, such as
    /// `IfcRelDeclares` in IFC2X3 or `IfcRelPositions` outside IFC4X3.
    EntityNotInSchema {
        /// The entity being authored.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A value for an attribute the release does not declare. It is
    /// refused rather than dropped.
    AuthoringNotInSchema {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A value the release's declaration of the attribute cannot hold,
    /// such as an IFC4 enumeration token IFC2X3 does not declare.
    AuthoringValueType {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// The type the release declares.
        declared: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The release requires an attribute the call leaves unset, such as
    /// the IFC2X3 `IfcRoot.OwnerHistory` (#202). It is refused rather than
    /// written as `$`; the `*_with_owner_history` writers take the
    /// `IfcOwnerHistory` IFC2X3 needs.
    AuthoringRequired {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute, by the release's own name.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A reference names an entity neither in the model nor staged.
    MissingReference {
        /// The entity being authored.
        entity: &'static str,
        /// The reference attribute.
        attribute: &'static str,
        /// The missing entity.
        target: EntityId,
    },
    /// A reference names an entity of a type the release does not accept
    /// there, such as an `OwnerHistory` that is not an `IfcOwnerHistory`.
    WrongReferenceType {
        /// The entity being authored.
        entity: &'static str,
        /// The reference attribute.
        attribute: &'static str,
        /// The referenced entity.
        target: EntityId,
        /// Its type.
        actual: String,
        /// The type expected.
        expected: &'static str,
    },
}

impl std::fmt::Display for SpatialAuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Invalid {
                entity,
                attribute,
                value,
            } => write!(f, "{entity}.{attribute}: {value}"),
            Self::MultipleSchemas { schemas } => write!(
                f,
                "the header declares {schemas} schemas; authoring binds to exactly one"
            ),
            Self::UnsupportedSchema { schema } => {
                write!(
                    f,
                    "the header declares {schema}, which has no bundled table"
                )
            }
            Self::EntityNotInSchema { entity, schema } => {
                write!(f, "{entity} is not an entity of {schema:?}")
            }
            Self::AuthoringNotInSchema {
                entity,
                attribute,
                schema,
            } => write!(
                f,
                "cannot author {entity}.{attribute}: not declared by {schema:?}"
            ),
            Self::AuthoringValueType {
                entity,
                attribute,
                declared,
                schema,
            } => write!(
                f,
                "cannot author {entity}.{attribute}: {schema:?} declares it {declared}"
            ),
            Self::AuthoringRequired {
                entity,
                attribute,
                schema,
            } => write!(f, "cannot author {entity}: {schema:?} requires {attribute}"),
            Self::MissingReference {
                entity,
                attribute,
                target,
            } => write!(f, "{entity}.{attribute} references missing {target}"),
            Self::WrongReferenceType {
                entity,
                attribute,
                target,
                actual,
                expected,
            } => write!(
                f,
                "{entity}.{attribute} references {target} of type {actual}; expected {expected}"
            ),
        }
    }
}

impl std::error::Error for SpatialAuthoringError {}

/// Result of staging a spatial record.
pub type SpatialAuthoringResult<T> = Result<T, SpatialAuthoringError>;

pub(crate) fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> SpatialAuthoringError {
    SpatialAuthoringError::Invalid {
        entity,
        attribute,
        value: value.into(),
    }
}

//! Why a type definition was refused.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

/// Why a type definition was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ElementTypeError {
    /// An attribute value the schema does not permit.
    Invalid {
        /// STEP type name.
        entity: &'static str,
        /// Attribute that was rejected.
        attribute: &'static str,
        /// The offending value.
        value: String,
    },
    /// The model's header declares several schemas; authoring binds to
    /// exactly one release.
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
    /// The model's release does not declare the type, or declares it
    /// abstract, such as `IfcDoorType` (IFC4 on) in an IFC2X3 model.
    EntityNotInSchema {
        /// STEP type name.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A value for an attribute the model's release does not declare, such
    /// as a `PredefinedType` for an IFC2X3 type that has none. It is
    /// refused rather than dropped.
    AuthoringNotInSchema {
        /// STEP type name.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The model's release requires an attribute the call leaves unset,
    /// such as the IFC2X3 `IfcRoot.OwnerHistory`.
    AuthoringRequired {
        /// STEP type name.
        entity: &'static str,
        /// The required attribute, as the release names it.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A referenced entity, such as the owner history, resolves neither in
    /// the model nor on the transaction.
    MissingEntity {
        /// The dangling id.
        id: EntityId,
    },
}

impl std::fmt::Display for ElementTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                write!(f, "{entity} is not an instantiable entity of {schema:?}")
            }
            Self::AuthoringNotInSchema {
                entity,
                attribute,
                schema,
            } => write!(
                f,
                "cannot author {entity}.{attribute}: not defined by {schema:?}"
            ),
            Self::AuthoringRequired {
                entity,
                attribute,
                schema,
            } => write!(f, "cannot author {entity}: {schema:?} requires {attribute}"),
            Self::MissingEntity { id } => write!(f, "entity #{} does not exist", id.0),
        }
    }
}

impl std::error::Error for ElementTypeError {}

/// Result of staging a type definition.
pub type ElementTypeResult<T> = Result<T, ElementTypeError>;

//! Why an occurrence was refused.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

/// Why an occurrence was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OccurrenceError {
    /// `GlobalId` did not parse as a 22-character IFC GUID.
    MalformedGuid {
        /// The offending value.
        value: String,
    },
    /// The token is not a member of this class's own enum.
    UnknownPredefinedType {
        /// STEP class.
        entity: &'static str,
        /// The offending token.
        token: String,
    },
    /// The class has no `PredefinedType` attribute at all.
    NoPredefinedType {
        /// STEP class.
        entity: &'static str,
    },
    /// `USERDEFINED` was given without an `ObjectType` naming it.
    UserDefinedWithoutObjectType {
        /// STEP class.
        entity: &'static str,
    },
    /// The occurrence was typed by a class the bound release does not pair
    /// with it.
    WrongTypeClass {
        /// STEP class of the occurrence.
        entity: &'static str,
        /// The only class the release permits, such as `IFCDOORTYPE` for an
        /// IFC4 `IfcDoor` or `IFCDOORSTYLE` for an IFC2X3 one.
        expected: &'static str,
        /// What the referenced entity actually is.
        found: String,
    },
    /// The class permits no type at all, but one was supplied.
    TypingNotPermitted {
        /// STEP class.
        entity: &'static str,
    },
    /// The bound release pairs no type class with this occurrence class,
    /// such as an IFC2X3 `IfcStair`, whose `IfcStairType` IFC2X3 does not
    /// declare, but one was supplied (#214).
    TypeClassNotInSchema {
        /// STEP class.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The typed-by reference does not resolve in the model.
    UnresolvedType {
        /// The dangling id.
        id: EntityId,
    },
    /// An enumeration token outside the enumeration the bound release
    /// declares for the attribute, such as a `ShapeType` or `BarRole`.
    UnknownToken {
        /// STEP class.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The offending token.
        token: String,
    },
    /// A measure the attribute's type does not admit: a non-positive or
    /// non-finite `IfcPositiveLengthMeasure`, or a non-finite
    /// `IfcAreaMeasure`.
    InvalidMeasure {
        /// STEP class.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The offending value, as written by `{:?}`.
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
    /// The model's release does not declare the class, or declares it
    /// abstract, such as `IfcBorehole` (IFC4X3 only) in an IFC4 model.
    EntityNotInSchema {
        /// STEP class.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A value for an attribute the model's release does not declare on the
    /// class. It is refused rather than dropped.
    AuthoringNotInSchema {
        /// STEP class.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The model's release requires an attribute the call leaves unset,
    /// such as the IFC2X3 `IfcRoot.OwnerHistory`.
    AuthoringRequired {
        /// STEP class.
        entity: &'static str,
        /// The required attribute, as the release names it.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The owner-history reference resolves neither in the model nor on the
    /// transaction.
    UnresolvedOwnerHistory {
        /// The dangling id.
        id: EntityId,
    },
    /// The owner-history reference is not an `IfcOwnerHistory`.
    NotAnOwnerHistory {
        /// The referenced id.
        id: EntityId,
        /// What the referenced entity actually is.
        found: String,
    },
}

impl std::fmt::Display for OccurrenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedGuid { value } => write!(f, "malformed GlobalId {value:?}"),
            Self::UnknownPredefinedType { entity, token } => {
                write!(f, "{entity}.PredefinedType: {token} is not a member")
            }
            Self::NoPredefinedType { entity } => write!(f, "{entity} has no PredefinedType"),
            Self::UserDefinedWithoutObjectType { entity } => {
                write!(f, "{entity}: USERDEFINED needs an ObjectType")
            }
            Self::WrongTypeClass {
                entity,
                expected,
                found,
            } => write!(f, "{entity} is typed by {expected}, not {found}"),
            Self::TypingNotPermitted { entity } => write!(f, "{entity} takes no type"),
            Self::TypeClassNotInSchema { entity, schema } => {
                write!(f, "{schema:?} pairs no type class with {entity}")
            }
            Self::UnresolvedType { id } => write!(f, "type #{} does not exist", id.0),
            Self::UnknownToken {
                entity,
                attribute,
                token,
            } => write!(f, "{entity}.{attribute}: {token} is not a member"),
            Self::InvalidMeasure {
                entity,
                attribute,
                value,
            } => write!(f, "{entity}.{attribute}: {value} is not admitted"),
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
            Self::UnresolvedOwnerHistory { id } => {
                write!(f, "owner history #{} does not exist", id.0)
            }
            Self::NotAnOwnerHistory { id, found } => {
                write!(f, "#{} is a {found}, not an IfcOwnerHistory", id.0)
            }
        }
    }
}

impl std::error::Error for OccurrenceError {}

/// Result alias for this crate.
pub type OccurrenceResult<T> = Result<T, OccurrenceError>;

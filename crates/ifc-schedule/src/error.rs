//! Malformed schedule data and traversal refusals.
//!
//! These describe a file that states something contradictory, not a failure of
//! this crate to read it. They are returned alongside results so one bad task
//! does not hide an otherwise readable schedule.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

pub use crate::sequence::SequenceCycle;
pub use crate::task::TaskTimeAnomaly;

/// An authored value rejected before it reached the model.
///
/// Distinct from the anomalies above: those describe a file already
/// written, this describes a draft refused so the file never says it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScheduleAuthoringError {
    /// An attribute value is not valid for its slot.
    InvalidValue {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute that failed.
        attribute: &'static str,
        /// What was expected instead.
        expected: &'static str,
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
    /// `IfcEvent` or `IfcWorkCalendar` in IFC2X3.
    EntityNotInSchema {
        /// The entity type being authored.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A draft supplied a value for an attribute the model's release does
    /// not declare, such as an IFC2X3 `IfcTask.PredefinedType`. It is
    /// refused rather than dropped.
    AuthoringNotInSchema {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A draft supplied a value the release's declaration cannot hold, such
    /// as text where IFC2X3 declares an `IfcDateTimeSelect`, a reference
    /// where it declares an `IfcTimeMeasure`, or a token outside its
    /// enumeration.
    AuthoringValueType {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute, by its IFC4 name.
        attribute: &'static str,
        /// The type the release declares.
        declared: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The release requires an attribute the call leaves unset, such as the
    /// IFC2X3 `IfcRoot.OwnerHistory` (#202) or `IfcTask.TaskId`. It is
    /// refused rather than written as `$`.
    AuthoringRequired {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute, by the release's own name.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// A reference, such as a caller-supplied `OwnerHistory`, names an
    /// entity that is neither in the model nor staged.
    MissingReference {
        /// The entity type being authored.
        entity: &'static str,
        /// The reference attribute.
        attribute: &'static str,
        /// The missing entity.
        target: EntityId,
    },
    /// A reference, such as a caller-supplied `OwnerHistory`, names an
    /// entity of a type the release does not accept there.
    WrongReferenceType {
        /// The entity type being authored.
        entity: &'static str,
        /// The reference attribute.
        attribute: &'static str,
        /// The referenced entity.
        target: EntityId,
        /// Its type.
        actual: String,
        /// The type the release accepts.
        expected: &'static str,
    },
}

impl std::fmt::Display for ScheduleAuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue {
                entity,
                attribute,
                expected,
            } => write!(f, "{entity}.{attribute}: expected {expected}"),
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
                "cannot author {entity}.{attribute}: not defined by {schema:?}"
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

impl std::error::Error for ScheduleAuthoringError {}

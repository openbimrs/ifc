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

/// Why a schedule read could not answer (#212).
///
/// The readers bind the release the model's header declares and find every
/// attribute by name in its table. A header they cannot bind is refused,
/// never read through another release's positions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScheduleReadError {
    /// The header declares several schemas; a read binds to exactly one.
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The header declares one schema the readers are not verified against
    /// (anything but IFC2X3, IFC4 and IFC4X3, so IFC4X1 and IFC4X2 too).
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written, or the release identifier.
        schema: String,
    },
    /// The sequence graph loops, so no ordering or downstream walk exists.
    Cycle(SequenceCycle),
    /// A sequence walk from `start` reached a chain longer than `limit`
    /// processes ([`MAX_SEQUENCE_DEPTH`](crate::sequence::MAX_SEQUENCE_DEPTH))
    /// and stopped (#236). The walk is refused rather than returned
    /// truncated, so a partial answer never reads as a complete one.
    SequenceDepthExceeded {
        /// The process the walk started from.
        start: EntityId,
        /// The depth budget that was reached.
        limit: usize,
    },
}

impl From<SequenceCycle> for ScheduleReadError {
    fn from(cycle: SequenceCycle) -> Self {
        Self::Cycle(cycle)
    }
}

impl std::fmt::Display for ScheduleReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleSchemas { schemas } => write!(
                f,
                "the header declares {schemas} schemas; a read binds to exactly one"
            ),
            Self::UnsupportedSchema { schema } => write!(
                f,
                "the header declares {schema}, which the schedule readers are not verified against"
            ),
            Self::Cycle(cycle) => write!(
                f,
                "the sequence graph returns to {} after {} steps",
                cycle.repeated,
                cycle.path.len()
            ),
            Self::SequenceDepthExceeded { start, limit } => write!(
                f,
                "the sequence walk from {start} exceeds the depth budget of {limit} processes"
            ),
        }
    }
}

impl std::error::Error for ScheduleReadError {}

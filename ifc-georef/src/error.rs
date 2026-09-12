//! Structured failures while resolving an IFC coordinate operation.

use ifc_model::EntityId;

/// Why a project-to-map operation could not be resolved losslessly.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum GeorefError {
    /// An entity referenced another that the model does not contain.
    MissingEntity {
        /// The entity holding the dangling reference.
        referrer: EntityId,
        /// The id that resolved to nothing.
        missing: EntityId,
    },
    /// An entity was not the IFC type the referencing slot requires.
    WrongType {
        /// The entity that was read.
        entity: EntityId,
        /// The IFC type the slot requires.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// A mandatory attribute was absent, so the value cannot be inferred.
    MissingAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// Zero-based slot index of the absent attribute.
        index: usize,
        /// Schema name of the attribute.
        name: &'static str,
    },
    /// An attribute was present but held the wrong kind of value.
    InvalidAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// Zero-based slot index of the offending attribute.
        index: usize,
        /// Schema name of the attribute.
        name: &'static str,
    },
    /// The coordinate operation is a subtype this crate does not resolve.
    ///
    /// Refused rather than approximated: a non-projected operation would
    /// silently produce wrong map coordinates.
    UnsupportedOperation {
        /// The coordinate operation entity.
        entity: EntityId,
        /// The IFC type actually declared.
        actual: String,
    },
    /// The map x axis is zero-length or non-finite, so no rotation exists.
    DegenerateAxis {
        /// The map conversion entity.
        entity: EntityId,
    },
    /// The map scale is zero, negative, or non-finite.
    InvalidScale {
        /// The map conversion entity.
        entity: EntityId,
        /// The rejected scale.
        value: f64,
    },
    /// The map unit is not a length unit this crate can reduce to metres.
    InvalidUnit {
        /// The unit entity.
        entity: EntityId,
        /// Which expectation failed.
        detail: &'static str,
    },
    /// A unit conversion chain revisited an entity or ran deeper than 16.
    ///
    /// Both guards exist: a repeated id catches a true loop, the depth cap
    /// catches an unbounded chain of distinct units.
    UnitCycle {
        /// The unit entity where the walk was abandoned.
        entity: EntityId,
    },
    /// The model header declares no `FILE_SCHEMA` token.
    MissingSchema,
    /// The header declares more than one schema, so the profile is ambiguous.
    ///
    /// Refused rather than picking one: the choice changes which coordinate
    /// operations are legal.
    AmbiguousSchema {
        /// Every schema token found in the header.
        tokens: Vec<String>,
    },
    /// The declared schema has no projected georeferencing profile here.
    ///
    /// IFC2X3 declares no georeferencing entities at all; other tokens may be
    /// unrecognised.
    UnsupportedSchema {
        /// The rejected schema token.
        token: String,
    },
    /// Chaining onto a project frame requires that frame to be supplied.
    MissingProjectFrame,
    /// The supplied project frame is not an invertible rigid transform.
    DegenerateProjectFrame,
    /// A direction was zero-length or non-finite, so it cannot be normalised.
    NonFiniteDirection {
        /// The direction entity.
        entity: EntityId,
    },
}

impl std::fmt::Display for GeorefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { referrer, missing } => {
                write!(f, "{referrer} references missing {missing}")
            }
            Self::WrongType {
                entity,
                expected,
                actual,
            } => write!(f, "{entity} is {actual}, expected {expected}"),
            Self::MissingAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} is missing {name} at slot {index}"),
            Self::InvalidAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} has invalid {name} at slot {index}"),
            Self::UnsupportedOperation { entity, actual } => {
                write!(f, "{entity} uses unsupported coordinate operation {actual}")
            }
            Self::DegenerateAxis { entity } => {
                write!(f, "{entity} has a zero-length or non-finite map x axis")
            }
            Self::InvalidScale { entity, value } => {
                write!(f, "{entity} has invalid map scale {value}")
            }
            Self::InvalidUnit { entity, detail } => {
                write!(f, "{entity} has unsupported or invalid map unit: {detail}")
            }
            Self::UnitCycle { entity } => {
                write!(f, "unit conversion chain at {entity} is cyclic or too deep")
            }
            Self::MissingSchema => {
                write!(f, "model header declares no FILE_SCHEMA token")
            }
            Self::AmbiguousSchema { tokens } => {
                write!(f, "model header declares multiple schemas: {tokens:?}")
            }
            Self::UnsupportedSchema { token } => {
                write!(
                    f,
                    "schema {token} does not declare IFC georeferencing entities \
                     or its coordinate-operation profile is not projected here"
                )
            }
            Self::MissingProjectFrame => {
                write!(
                    f,
                    "chaining a project-to-map operation onto a project frame \
                     requires that frame to be supplied explicitly"
                )
            }
            Self::DegenerateProjectFrame => {
                write!(
                    f,
                    "supplied project frame is not an invertible rigid transform \
                     (zero or non-finite determinant)"
                )
            }
            Self::NonFiniteDirection { entity } => {
                write!(f, "{entity} has a non-finite or zero-length direction")
            }
        }
    }
}

impl std::error::Error for GeorefError {}

/// Result of a georeferencing read, carrying [`GeorefError`] on failure.
pub type GeorefResult<T> = Result<T, GeorefError>;

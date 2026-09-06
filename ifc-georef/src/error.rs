//! Structured failures while resolving an IFC coordinate operation.

use ifc_model::EntityId;

/// Why a project-to-map operation could not be resolved losslessly.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum GeorefError {
    MissingEntity {
        referrer: EntityId,
        missing: EntityId,
    },
    WrongType {
        entity: EntityId,
        expected: &'static str,
        actual: String,
    },
    MissingAttribute {
        entity: EntityId,
        index: usize,
        name: &'static str,
    },
    InvalidAttribute {
        entity: EntityId,
        index: usize,
        name: &'static str,
    },
    UnsupportedOperation {
        entity: EntityId,
        actual: String,
    },
    DegenerateAxis {
        entity: EntityId,
    },
    InvalidScale {
        entity: EntityId,
        value: f64,
    },
    InvalidUnit {
        entity: EntityId,
        detail: &'static str,
    },
    UnitCycle {
        entity: EntityId,
    },
    MissingSchema,
    AmbiguousSchema {
        tokens: Vec<String>,
    },
    UnsupportedSchema {
        token: String,
    },
    MissingProjectFrame,
    DegenerateProjectFrame,
    NonFiniteDirection {
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

pub type GeorefResult<T> = Result<T, GeorefError>;

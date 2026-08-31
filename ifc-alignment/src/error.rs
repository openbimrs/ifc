//! Structured alignment interpretation failures.

use ifc_model::EntityId;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentError {
    MissingEntity {
        entity: EntityId,
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
    InvalidUnits {
        detail: &'static str,
    },
    InvalidSegment {
        entity: EntityId,
        detail: &'static str,
    },
    Unsupported {
        entity: EntityId,
        type_name: String,
        detail: &'static str,
    },
    Graph {
        detail: String,
    },
}

pub type AlignmentResult<T> = Result<T, AlignmentError>;

impl std::fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { entity } => write!(f, "missing alignment entity {entity}"),
            Self::WrongType {
                entity,
                expected,
                actual,
            } => write!(f, "{entity} is {actual}; expected {expected}"),
            Self::MissingAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} misses {name} at slot {index}"),
            Self::InvalidAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} has invalid {name} at slot {index}"),
            Self::InvalidUnits { detail } => write!(f, "invalid alignment units: {detail}"),
            Self::InvalidSegment { entity, detail } => {
                write!(f, "invalid alignment segment {entity}: {detail}")
            }
            Self::Unsupported {
                entity,
                type_name,
                detail,
            } => write!(f, "unsupported {type_name} at {entity}: {detail}"),
            Self::Graph { detail } => write!(f, "invalid neutral alignment graph: {detail}"),
        }
    }
}

impl std::error::Error for AlignmentError {}

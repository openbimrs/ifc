//! Structured failures for resource projections, queries, and authoring.

use ifc_model::EntityId;

pub type ResourceResult<T> = Result<T, ResourceError>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceError {
    MissingSchema,
    AmbiguousSchema {
        tokens: Vec<String>,
    },
    UnsupportedSchema {
        token: String,
    },
    EntityNotFound {
        id: EntityId,
    },
    WrongType {
        id: EntityId,
        expected: &'static str,
        actual: String,
    },
    MissingAttribute {
        entity: EntityId,
        attribute: &'static str,
    },
    InvalidValue {
        entity: EntityId,
        attribute: &'static str,
        expected: &'static str,
    },
    InvalidEnumeration {
        entity: Option<EntityId>,
        attribute: &'static str,
        value: String,
    },
    DanglingReference {
        entity: EntityId,
        attribute: &'static str,
        target: EntityId,
    },
    WrongReferenceType {
        entity: EntityId,
        attribute: &'static str,
        target: EntityId,
        expected: &'static str,
        actual: String,
    },
    InvalidCardinality {
        entity: EntityId,
        attribute: &'static str,
        minimum: usize,
        actual: usize,
    },
    DuplicateReference {
        entity: EntityId,
        attribute: &'static str,
        target: EntityId,
    },
    SemanticViolation {
        entity: Option<EntityId>,
        rule: &'static str,
    },
    Cycle {
        at: EntityId,
    },
    BudgetExceeded {
        max_depth: usize,
        max_nodes: usize,
    },
    InvalidDraft {
        entity_type: &'static str,
        attribute: &'static str,
        expected: &'static str,
    },
    InvalidGlobalId,
    TransactionConflict {
        expected: u64,
        actual: u64,
    },
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => f.write_str("the model header declares no IFC schema"),
            Self::AmbiguousSchema { tokens } => {
                write!(f, "the model declares multiple schemas: {tokens:?}")
            }
            Self::UnsupportedSchema { token } => {
                write!(
                    f,
                    "unsupported resource schema `{token}`; expected IFC4 ADD2 TC1"
                )
            }
            Self::EntityNotFound { id } => write!(f, "entity {id} does not exist"),
            Self::WrongType {
                id,
                expected,
                actual,
            } => {
                write!(f, "entity {id} is {actual}, expected {expected}")
            }
            Self::MissingAttribute { entity, attribute } => {
                write!(f, "entity {entity} has no `{attribute}` slot")
            }
            Self::InvalidValue {
                entity,
                attribute,
                expected,
            } => {
                write!(
                    f,
                    "entity {entity}.{attribute} is invalid; expected {expected}"
                )
            }
            Self::InvalidEnumeration {
                entity,
                attribute,
                value,
            } => match entity {
                Some(entity) => write!(
                    f,
                    "entity {entity}.{attribute} has undeclared enumeration `{value}`"
                ),
                None => write!(
                    f,
                    "resource draft.{attribute} has undeclared enumeration `{value}`"
                ),
            },
            Self::DanglingReference {
                entity,
                attribute,
                target,
            } => {
                write!(f, "entity {entity}.{attribute} refers to missing {target}")
            }
            Self::WrongReferenceType {
                entity,
                attribute,
                target,
                expected,
                actual,
            } => write!(
                f,
                "entity {entity}.{attribute} refers to {target} ({actual}), expected {expected}"
            ),
            Self::InvalidCardinality {
                entity,
                attribute,
                minimum,
                actual,
            } => write!(
                f,
                "entity {entity}.{attribute} has {actual} items; minimum is {minimum}"
            ),
            Self::DuplicateReference {
                entity,
                attribute,
                target,
            } => write!(f, "entity {entity}.{attribute} repeats reference {target}"),
            Self::SemanticViolation { rule, .. } => {
                write!(f, "resource semantic rule `{rule}` failed")
            }
            Self::Cycle { at } => write!(f, "resource composition cycle revisits {at}"),
            Self::BudgetExceeded {
                max_depth,
                max_nodes,
            } => write!(
                f,
                "resource traversal exceeded depth {max_depth} or node budget {max_nodes}"
            ),
            Self::InvalidDraft {
                entity_type,
                attribute,
                expected,
            } => write!(
                f,
                "draft {entity_type}.{attribute} is invalid; expected {expected}"
            ),
            Self::InvalidGlobalId => {
                f.write_str("GlobalId is not a valid 22-character compressed IFC GUID")
            }
            Self::TransactionConflict { expected, actual } => write!(
                f,
                "resource transaction revision conflict: expected {expected}, actual {actual}"
            ),
        }
    }
}

impl std::error::Error for ResourceError {}

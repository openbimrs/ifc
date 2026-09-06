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
    /// No `FILE_SCHEMA` token was declared.
    MissingSchema,
    /// More than one `FILE_SCHEMA` token was declared.
    AmbiguousSchema {
        tokens: Vec<String>,
    },
    /// The declared schema is not one this crate can interpret.
    ///
    /// Alignment entities (`IfcAlignment*`) were introduced in IFC4X3; IFC2X3
    /// and IFC4 ADD2 TC1 do not declare them at all, so there is no version
    /// dispatch here the way `ifc-resource`/`ifc-structural` have one --
    /// exactly one profile is authoritative and anything else is refused.
    UnsupportedSchema {
        token: String,
    },
    /// A relationship or nesting structure violates a stated invariant.
    SemanticViolation {
        entity: Option<EntityId>,
        rule: &'static str,
    },
    /// A dangling reference: the target id is not present in the model.
    DanglingReference {
        entity: EntityId,
        attribute: &'static str,
        target: EntityId,
    },
    /// Traversal exceeded an explicit bound.
    BudgetExceeded {
        max_depth: usize,
        max_nodes: usize,
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
            Self::MissingSchema => write!(f, "no FILE_SCHEMA token was declared"),
            Self::AmbiguousSchema { tokens } => {
                write!(f, "ambiguous FILE_SCHEMA tokens: {tokens:?}")
            }
            Self::UnsupportedSchema { token } => write!(
                f,
                "unsupported schema {token}: alignment entities require IFC4X3 ADD2"
            ),
            Self::SemanticViolation { entity, rule } => match entity {
                Some(entity) => write!(f, "{entity} violates {rule}"),
                None => write!(f, "violates {rule}"),
            },
            Self::DanglingReference {
                entity,
                attribute,
                target,
            } => write!(f, "{entity}.{attribute} references missing {target}"),
            Self::BudgetExceeded {
                max_depth,
                max_nodes,
            } => write!(
                f,
                "traversal exceeded bounds (max_depth={max_depth}, max_nodes={max_nodes})"
            ),
        }
    }
}

impl std::error::Error for AlignmentError {}

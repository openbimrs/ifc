//! Structured failures for structural analysis projections and authoring.

use ifc_model::EntityId;

/// Result type for `ifc-structural` operations.
pub type StructuralResult<T> = Result<T, StructuralError>;

/// Why a structural projection or draft was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructuralError {
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
    MissingRequired {
        entity_type: String,
        attribute: String,
    },
    UnsupportedAttribute {
        entity_type: String,
        attribute: String,
    },
    InvalidValue {
        entity: EntityId,
        attribute: &'static str,
        expected: &'static str,
    },
    InvalidDraftValue {
        entity_type: &'static str,
        attribute: &'static str,
        expected: &'static str,
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
        maximum: Option<usize>,
        actual: usize,
    },
    SemanticViolation {
        entity: Option<EntityId>,
        rule: &'static str,
    },
    InvalidGlobalId,
}

impl std::fmt::Display for StructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => f.write_str("the model header declares no IFC schema"),
            Self::AmbiguousSchema { tokens } => write!(f, "the model declares multiple schemas: {tokens:?}"),
            Self::UnsupportedSchema { token } => write!(f, "unsupported IFC schema `{token}`"),
            Self::EntityNotFound { id } => write!(f, "entity {id} does not exist"),
            Self::WrongType { id, expected, actual } => write!(f, "entity {id} is {actual}, expected {expected}"),
            Self::MissingAttribute { entity, attribute } => write!(f, "entity {entity} has no `{attribute}` attribute in the selected schema"),
            Self::MissingRequired { entity_type, attribute } => write!(f, "{entity_type}.{attribute} is required"),
            Self::UnsupportedAttribute { entity_type, attribute } => write!(f, "{entity_type}.{attribute} does not exist in the selected schema"),
            Self::InvalidValue { entity, attribute, expected } => write!(f, "entity {entity} has invalid `{attribute}`; expected {expected}"),
            Self::InvalidDraftValue { entity_type, attribute, expected } => write!(f, "draft {entity_type} has invalid `{attribute}`; expected {expected}"),
            Self::DanglingReference { entity, attribute, target } => write!(f, "entity {entity}.{attribute} refers to missing {target}"),
            Self::WrongReferenceType { entity, attribute, target, expected, actual } => write!(f, "entity {entity}.{attribute} refers to {target} ({actual}), expected {expected}"),
            Self::InvalidCardinality { entity, attribute, minimum, maximum, actual } => write!(f, "entity {entity}.{attribute} cardinality {actual} is outside {minimum}..{maximum:?}"),
            Self::SemanticViolation { rule, .. } => write!(f, "structural semantic rule `{rule}` failed"),
            Self::InvalidGlobalId => f.write_str("GlobalId is not a valid 22-character IFC GUID"),
        }
    }
}

impl std::error::Error for StructuralError {}

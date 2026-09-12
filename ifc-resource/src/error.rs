//! Structured failures for resource projections, queries, and authoring.

use ifc_model::EntityId;

/// Result of a resource projection, query, or authoring call, carrying
/// [`ResourceError`] on failure.
pub type ResourceResult<T> = Result<T, ResourceError>;

/// Why a resource projection, query, or authoring call could not be
/// completed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceError {
    /// The model header declares no `FILE_SCHEMA` token.
    MissingSchema,
    /// The header declares more than one schema, so the profile is ambiguous.
    AmbiguousSchema {
        /// Every schema token found in the header.
        tokens: Vec<String>,
    },
    /// The declared schema is not IFC4 ADD2 TC1 or IFC4X3 ADD2.
    UnsupportedSchema {
        /// The rejected schema token.
        token: String,
    },
    /// A requested entity id does not exist in the model.
    EntityNotFound {
        /// The missing entity id.
        id: EntityId,
    },
    /// An entity was not the IFC type the caller requested.
    WrongType {
        /// The entity that was read.
        id: EntityId,
        /// The IFC type the caller expected.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// The schema declares no attribute of this name on the entity type.
    MissingAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// Schema name of the absent attribute.
        attribute: &'static str,
    },
    /// An attribute was present but held the wrong kind of value.
    InvalidValue {
        /// The entity that was read.
        entity: EntityId,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// What kind of value was expected.
        expected: &'static str,
    },
    /// An enumeration attribute held a value the schema does not declare.
    InvalidEnumeration {
        /// The entity that was read, when the value came from an entity
        /// rather than an authoring draft.
        entity: Option<EntityId>,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// The undeclared value.
        value: String,
    },
    /// An entity reference attribute pointed at an id the model does not
    /// contain.
    DanglingReference {
        /// The entity holding the dangling reference.
        entity: EntityId,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// The id that resolved to nothing.
        target: EntityId,
    },
    /// An entity reference resolved, but the target is not the IFC type the
    /// slot requires.
    WrongReferenceType {
        /// The entity holding the reference.
        entity: EntityId,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// The referenced entity.
        target: EntityId,
        /// The IFC type the slot requires.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// An aggregate attribute held fewer items than the schema's minimum
    /// cardinality.
    InvalidCardinality {
        /// The entity that was read.
        entity: EntityId,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// The schema's minimum cardinality.
        minimum: usize,
        /// The actual number of items found.
        actual: usize,
    },
    /// A `SET` attribute repeated the same reference more than once.
    DuplicateReference {
        /// The entity that was read.
        entity: EntityId,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// The repeated reference.
        target: EntityId,
    },
    /// A WHERE rule or other cross-attribute IFC semantic constraint failed.
    SemanticViolation {
        /// The entity that violated the rule, when applicable.
        entity: Option<EntityId>,
        /// Name of the violated rule.
        rule: &'static str,
    },
    /// A resource composition or nesting traversal revisited an entity.
    Cycle {
        /// The entity where the cycle was detected.
        at: EntityId,
    },
    /// A bounded traversal exceeded its depth or node budget.
    BudgetExceeded {
        /// The maximum depth allowed.
        max_depth: usize,
        /// The maximum number of nodes allowed.
        max_nodes: usize,
    },
    /// A staged authoring draft could not be committed as written.
    InvalidDraft {
        /// The IFC entity type being authored.
        entity_type: &'static str,
        /// Schema name of the offending attribute.
        attribute: &'static str,
        /// What kind of value was expected.
        expected: &'static str,
    },
    /// A supplied `GlobalId` is not a valid 22-character compressed IFC GUID.
    InvalidGlobalId,
    /// A transaction commit conflicted with a concurrent revision.
    TransactionConflict {
        /// The revision the transaction expected.
        expected: u64,
        /// The revision actually current.
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
                    "unsupported resource schema `{token}`; expected IFC4 ADD2 TC1 or IFC4X3 ADD2"
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

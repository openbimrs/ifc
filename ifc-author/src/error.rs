//! Why a construction attempt was refused.
//!
//! Every variant names the entity and, where meaningful, the attribute, because
//! an authoring failure is a programming error in the *calling* application and
//! the message is the whole diagnostic.

use std::fmt;

use ifc_model::EntityId;

/// A refused construction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorError {
    /// The requested entity is not present in the model snapshot.
    MissingEntity {
        /// The missing entity id.
        id: EntityId,
    },
    /// The schema does not declare this entity type.
    ///
    /// Reported rather than ignored: silently accepting an unknown type is how
    /// a typo becomes a file other tools reject.
    UnknownEntity {
        /// The requested type name.
        entity: String,
    },
    /// The schema declares the entity, but not this attribute.
    UnknownAttribute {
        /// The entity being built.
        entity: String,
        /// The attribute name the caller supplied.
        attribute: String,
        /// Attribute names the schema does declare, in positional order.
        known: Vec<String>,
    },
    /// A required (non-`OPTIONAL`) attribute was never set.
    MissingRequired {
        /// The entity being built.
        entity: String,
        /// The unset attribute.
        attribute: String,
    },
    /// The same attribute was set twice.
    ///
    /// A silent overwrite hides a copy-paste error in the caller.
    DuplicateAttribute {
        /// The entity being built.
        entity: String,
        /// The attribute set more than once.
        attribute: String,
    },
    /// The value does not match the attribute's declared type.
    TypeMismatch {
        /// The entity being built.
        entity: String,
        /// The attribute that was set.
        attribute: String,
        /// The type the schema declares.
        expected: String,
        /// What the supplied value actually was.
        found: String,
    },
    /// A scalar was supplied where the schema declares an aggregate, or vice
    /// versa.
    AggregateMismatch {
        /// The entity being built.
        entity: String,
        /// The attribute that was set.
        attribute: String,
        /// Whether the schema declares an aggregate.
        expected_aggregate: bool,
    },
    /// A GlobalId was supplied that is not a valid 22-character IFC GUID.
    InvalidGlobalId {
        /// The entity being built.
        entity: String,
        /// The rejected text.
        found: String,
    },
    /// An existing entity does not have the arity declared by the schema.
    ArityMismatch {
        /// The entity being edited.
        entity: String,
        /// Number of attributes declared by the schema.
        expected: usize,
        /// Number of attributes present in the model.
        found: usize,
    },
}

impl fmt::Display for AuthorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntity { id } => write!(f, "model has no entity `{id}`"),
            Self::UnknownEntity { entity } => {
                write!(f, "schema does not declare entity `{entity}`")
            }
            Self::UnknownAttribute {
                entity,
                attribute,
                known,
            } => write!(
                f,
                "`{entity}` has no attribute `{attribute}`; declared: {}",
                known.join(", ")
            ),
            Self::MissingRequired { entity, attribute } => write!(
                f,
                "`{entity}` requires attribute `{attribute}`, which was not set"
            ),
            Self::DuplicateAttribute { entity, attribute } => {
                write!(f, "`{entity}` attribute `{attribute}` was set twice")
            }
            Self::TypeMismatch {
                entity,
                attribute,
                expected,
                found,
            } => write!(
                f,
                "`{entity}.{attribute}` expects {expected}, found {found}"
            ),
            Self::AggregateMismatch {
                entity,
                attribute,
                expected_aggregate,
            } => {
                let (want, got) = if *expected_aggregate {
                    ("an aggregate", "a scalar")
                } else {
                    ("a scalar", "an aggregate")
                };
                write!(f, "`{entity}.{attribute}` expects {want}, found {got}")
            }
            Self::InvalidGlobalId { entity, found } => write!(
                f,
                "`{entity}.GlobalId` must be a 22-character IFC GUID, found `{found}`"
            ),
            Self::ArityMismatch {
                entity,
                expected,
                found,
            } => write!(
                f,
                "`{entity}` declares {expected} attributes, but the model contains {found}"
            ),
        }
    }
}

impl std::error::Error for AuthorError {}

/// The result of a construction attempt.
pub type AuthorResult<T> = Result<T, AuthorError>;

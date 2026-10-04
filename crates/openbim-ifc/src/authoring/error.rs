//! Why a batch was refused.

use std::fmt;

use ifc_model::{Conflict, EntityId};

/// A refused batch: which operation failed, and why. Nothing was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AuthoringError {
    /// The operation's position in the batch; `None` for a failure of the
    /// batch as a whole (the declared release, the commit).
    pub op: Option<usize>,
    /// What was refused; boxed, as the variants carry their diagnostics.
    pub failure: Box<AuthoringFailure>,
}

/// What an [`AuthoringError`] refused.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AuthoringFailure {
    /// The declared release has no entity of this name.
    UnknownEntity {
        /// The release, e.g. `IFC4`.
        schema: String,
        /// The type as asked for.
        type_name: String,
    },
    /// The entity type is `ABSTRACT`: no instance of it can exist.
    AbstractEntity {
        /// The type, in the schema's spelling.
        type_name: String,
    },
    /// An operation's type argument is not of the kind it builds, such as
    /// a `Product` of `IfcBuildingStorey`.
    NotA {
        /// The type as asked for.
        type_name: String,
        /// The kind the operation builds, e.g. `IfcProduct`.
        expected: String,
    },
    /// `ifc-author` refused the record: an unknown, duplicate, derived or
    /// missing attribute, or a value of the wrong type or form.
    Author(ifc_author::AuthorError),
    /// An aggregate value breaks its declared bounds or uniqueness.
    Cardinality {
        /// The entity being written.
        entity: String,
        /// The attribute.
        attribute: String,
        /// What is wrong, e.g. `0 items; at least 1`.
        detail: String,
    },
    /// The entity an operation edits or removes does not exist.
    MissingEntity(EntityId),
    /// A value references an entity that does not exist.
    MissingReference {
        /// The entity being written.
        entity: String,
        /// The attribute holding the reference.
        attribute: String,
        /// The missing entity.
        target: EntityId,
    },
    /// A value references an entity of a type the attribute does not
    /// accept.
    WrongReferenceType {
        /// The entity being written.
        entity: String,
        /// The attribute holding the reference.
        attribute: String,
        /// The referenced entity.
        target: EntityId,
        /// Its type.
        actual: String,
        /// The declared type.
        expected: String,
    },
    /// A `GlobalId` another entity already holds.
    DuplicateGlobalId {
        /// The `GlobalId`.
        global_id: String,
        /// The entity holding it.
        holder: EntityId,
    },
    /// The operation would give an object a second relationship the
    /// schema allows once: a second containment (`ContainedInStructure`),
    /// decomposition (`Decomposes`) or type (`IsTypedBy`), or a second
    /// `IfcProject`.
    AlreadyRelated {
        /// The object.
        id: EntityId,
        /// The relationship type, e.g. `IFCRELCONTAINEDINSPATIALSTRUCTURE`.
        relationship: &'static str,
        /// The relationship (or project) it already has.
        existing: EntityId,
    },
    /// A relationship would relate an entity to itself, or to nothing.
    InvalidRelationship {
        /// The relationship type.
        relationship: &'static str,
        /// What is wrong.
        detail: &'static str,
    },
    /// A removal is blocked: an entity that is not a relationship still
    /// references the target. Remove or edit the referrer first.
    StillReferenced {
        /// The entity being removed.
        id: EntityId,
        /// The entity referencing it.
        by: EntityId,
    },
    /// A handle names an operation that has not run yet or produced no
    /// entity.
    InvalidHandle {
        /// The handle as passed.
        handle: u64,
        /// What is wrong.
        detail: String,
    },
    /// Placement coordinates or directions the schema cannot hold.
    InvalidPlacement {
        /// What is wrong.
        detail: String,
    },
    /// The model's ids reach the handle range, so a reference could not
    /// be told from a handle.
    IdRangeExhausted {
        /// The model's next free id.
        next_id: u64,
    },
    /// The planned transaction failed its preflight.
    Conflict(Vec<Conflict>),
}

impl AuthoringError {
    pub(super) fn at(op: Option<usize>, failure: AuthoringFailure) -> Self {
        Self {
            op,
            failure: Box::new(failure),
        }
    }
}

impl fmt::Display for AuthoringFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEntity { schema, type_name } => {
                write!(f, "{schema} does not declare entity `{type_name}`")
            }
            Self::AbstractEntity { type_name } => {
                write!(f, "`{type_name}` is abstract and cannot be instantiated")
            }
            Self::NotA {
                type_name,
                expected,
            } => write!(f, "`{type_name}` is not an {expected}"),
            Self::Author(error) => error.fmt(f),
            Self::Cardinality {
                entity,
                attribute,
                detail,
            } => write!(f, "`{entity}.{attribute}`: {detail}"),
            Self::MissingEntity(id) => write!(f, "no entity {id}"),
            Self::MissingReference {
                entity,
                attribute,
                target,
            } => write!(f, "`{entity}.{attribute}` references missing {target}"),
            Self::WrongReferenceType {
                entity,
                attribute,
                target,
                actual,
                expected,
            } => write!(
                f,
                "`{entity}.{attribute}` references {target}, an {actual}; it accepts {expected}"
            ),
            Self::DuplicateGlobalId { global_id, holder } => {
                write!(f, "GlobalId `{global_id}` is already held by {holder}")
            }
            Self::AlreadyRelated {
                id,
                relationship,
                existing,
            } => write!(
                f,
                "{id} already has its one {relationship} ({existing}); the schema allows one"
            ),
            Self::InvalidRelationship {
                relationship,
                detail,
            } => write!(f, "{relationship}: {detail}"),
            Self::StillReferenced { id, by } => write!(
                f,
                "{id} is still referenced by {by}, which is not a relationship; \
                 remove or edit it first"
            ),
            Self::InvalidHandle { handle, detail } => {
                write!(f, "handle {handle}: {detail}")
            }
            Self::InvalidPlacement { detail } => write!(f, "placement: {detail}"),
            Self::IdRangeExhausted { next_id } => write!(
                f,
                "the model's next id {next_id} reaches the handle range; \
                 references could not be told from handles"
            ),
            Self::Conflict(conflicts) => write!(
                f,
                "the planned transaction failed its preflight: {conflicts:?}"
            ),
        }
    }
}

impl fmt::Display for AuthoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.op {
            Some(op) => write!(f, "op {op}: {}", self.failure),
            None => self.failure.fmt(f),
        }
    }
}

impl std::error::Error for AuthoringError {}

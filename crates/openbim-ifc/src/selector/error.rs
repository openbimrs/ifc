//! Why a selector or value path was refused: [`ParseError`] before
//! evaluation, [`SelectorError`] during it.

use std::fmt;
use std::ops::Range;

use ifc_model::{Conflict, EntityId};

use crate::{DuplicateGlobalId, NamedAttributeError};

/// A selector or value path that does not follow the grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    /// What is wrong.
    pub kind: ParseErrorKind,
    /// The byte range of the offending text; empty at the end of input.
    pub span: Range<usize>,
}

/// What a [`ParseError`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The text holds no facet (or no key), only whitespace and comments.
    Empty,
    /// The input ended where the grammar expects more.
    UnexpectedEnd {
        /// What was expected.
        expected: &'static str,
    },
    /// A character the grammar does not allow here.
    UnexpectedChar {
        /// The character found.
        found: char,
        /// What was expected.
        expected: &'static str,
    },
    /// A `"` string with no closing quote.
    UnterminatedString,
    /// A `/` pattern with no closing slash.
    UnterminatedPattern,
    /// A `/*` comment with no closing `*/`.
    UnterminatedComment,
    /// A pattern the regular-expression compiler refuses, or `//`.
    InvalidPattern(String),
    /// A lower-case word before a comparison that is no keyword (`type`,
    /// `material`, `classification`, `location`, `container`, `group`).
    UnknownKeyword(String),
    /// An attribute name that is not a letter followed by letters, digits
    /// and underscores.
    InvalidAttributeName(String),
    /// A word standing alone that is neither a class (`Ifc...`) nor a
    /// `GlobalId`.
    ExpectedClassOrGlobalId(String),
    /// `NULL`, `TRUE`, `FALSE` or a pattern after an operator other than
    /// `=` and `!=`.
    OperatorNotApplicable {
        /// The operator as written.
        operator: String,
        /// The value as written.
        value: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at bytes {}..{}",
            self.kind, self.span.start, self.span.end
        )
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty selector"),
            Self::UnexpectedEnd { expected } => write!(f, "unexpected end, expected {expected}"),
            Self::UnexpectedChar { found, expected } => {
                write!(f, "unexpected {found:?}, expected {expected}")
            }
            Self::UnterminatedString => f.write_str("unterminated string"),
            Self::UnterminatedPattern => f.write_str("unterminated /pattern/"),
            Self::UnterminatedComment => f.write_str("unterminated /* comment"),
            Self::InvalidPattern(detail) => write!(f, "invalid pattern: {detail}"),
            Self::UnknownKeyword(word) => write!(
                f,
                "unknown keyword {word:?} (type, material, classification, location, \
                 container, group; attribute names start upper-case)"
            ),
            Self::InvalidAttributeName(name) => write!(f, "invalid attribute name {name:?}"),
            Self::ExpectedClassOrGlobalId(word) => write!(
                f,
                "{word:?} is neither a class (Ifc...) nor a GlobalId, and no comparison follows"
            ),
            Self::OperatorNotApplicable { operator, value } => {
                write!(f, "{operator} does not apply to {value}; use = or !=")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Why a selector, a value path or a write by path was refused. Nothing is
/// guessed and, for a write, nothing is written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SelectorError {
    /// A class facet names no entity of the release.
    UnknownClass {
        /// The class as written.
        name: String,
        /// The facet's byte range.
        span: Range<usize>,
    },
    /// An attribute facet names an attribute no entity of the release
    /// declares.
    UnknownAttribute {
        /// The attribute as written.
        name: String,
        /// The facet's byte range.
        span: Range<usize>,
    },
    /// An entity the call names is not in the model.
    MissingEntity(EntityId),
    /// An instance facet names a `GlobalId` several entities carry.
    DuplicateGlobalId(DuplicateGlobalId),
    /// A relationship's end is not the reference (or list of references)
    /// the schema declares.
    Malformed {
        /// The relationship or resource entity.
        entity: EntityId,
        /// The attribute that is malformed.
        attribute: &'static str,
    },
    /// A relationship names an entity the model lacks.
    MissingReference {
        /// The relationship.
        relationship: EntityId,
        /// The id it names.
        target: EntityId,
    },
    /// The schema allows one answer and the file states several: two types
    /// for one object, two containers or two aggregates.
    Ambiguous {
        /// The entity asked about.
        entity: EntityId,
        /// What is ambiguous (`type`, `container`, `aggregate`).
        relation: &'static str,
        /// The different answers, in file order.
        candidates: Vec<EntityId>,
    },
    /// The exact property resolver refused the model or an entity.
    #[cfg(feature = "properties")]
    Property {
        /// The entity whose properties were read; `None` for the model.
        entity: Option<EntityId>,
        /// Why.
        error: ifc_properties::ExactPropertyError,
    },
    /// A property value of a form the selector does not compare or
    /// return: a table value.
    UnsupportedValue {
        /// The entity whose property it is.
        entity: EntityId,
        /// The form (`table`).
        form: &'static str,
    },
    /// The operation needs a facade feature this build leaves out
    /// (`properties` for a property facet or path).
    FeatureDisabled(&'static str),
    /// A write's path does not name something a value can be written to:
    /// a pattern, a computed key (`count`, `class`, `id`), or a prefix that
    /// does not lead to entities.
    NotSettable {
        /// The path as written.
        path: String,
        /// Why.
        reason: String,
    },
    /// A by-name attribute read or write was refused.
    Attribute(NamedAttributeError),
    /// A value does not fit the attribute's declared type in the release.
    TypeMismatch {
        /// The entity written.
        entity: EntityId,
        /// The attribute.
        attribute: String,
        /// Why.
        detail: String,
    },
    /// A property write carries a bare value and no property of that name
    /// states the type to wrap it in; pass a typed value
    /// (`IFCLABEL('F90')`).
    UntypedValue {
        /// The object written.
        object: EntityId,
        /// The set name.
        set: String,
        /// The property name.
        property: String,
    },
    /// The property edit was refused.
    #[cfg(feature = "properties")]
    PropertyEdit(crate::PropertyEditError),
    /// The planned transaction failed its preflight.
    Conflict(Vec<Conflict>),
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownClass { name, span } => write!(
                f,
                "the release declares no entity {name:?} (bytes {}..{})",
                span.start, span.end
            ),
            Self::UnknownAttribute { name, span } => write!(
                f,
                "no entity of the release declares an attribute {name:?} (bytes {}..{})",
                span.start, span.end
            ),
            Self::MissingEntity(EntityId(id)) => write!(f, "no entity #{id}"),
            Self::DuplicateGlobalId(duplicate) => write!(f, "{duplicate}"),
            Self::Malformed { entity, attribute } => write!(
                f,
                "#{}'s {attribute} is not the reference the schema declares",
                entity.0
            ),
            Self::MissingReference {
                relationship,
                target,
            } => write!(
                f,
                "#{} names #{}, which is not in the model",
                relationship.0, target.0
            ),
            Self::Ambiguous {
                entity,
                relation,
                candidates,
            } => {
                write!(f, "#{} has {} {relation}s:", entity.0, candidates.len())?;
                for EntityId(id) in candidates {
                    write!(f, " #{id}")?;
                }
                Ok(())
            }
            #[cfg(feature = "properties")]
            Self::Property { entity, error } => match entity {
                Some(EntityId(id)) => write!(f, "properties of #{id}: {error}"),
                None => write!(f, "properties: {error}"),
            },
            Self::UnsupportedValue { entity, form } => write!(
                f,
                "#{} holds a {form} value, which a selector does not compare",
                entity.0
            ),
            Self::FeatureDisabled(feature) => {
                write!(f, "this build leaves out the `{feature}` feature")
            }
            Self::NotSettable { path, reason } => write!(f, "cannot write {path:?}: {reason}"),
            Self::Attribute(error) => write!(f, "{error}"),
            Self::TypeMismatch {
                entity,
                attribute,
                detail,
            } => write!(f, "#{}.{attribute}: {detail}", entity.0),
            Self::UntypedValue {
                object,
                set,
                property,
            } => write!(
                f,
                "#{} states no {set}.{property} to take the type from; pass a typed value",
                object.0
            ),
            #[cfg(feature = "properties")]
            Self::PropertyEdit(error) => write!(f, "{error}"),
            Self::Conflict(conflicts) => write!(f, "the planned write conflicts: {conflicts:?}"),
        }
    }
}

impl std::error::Error for SelectorError {}

impl From<NamedAttributeError> for SelectorError {
    fn from(error: NamedAttributeError) -> Self {
        Self::Attribute(error)
    }
}

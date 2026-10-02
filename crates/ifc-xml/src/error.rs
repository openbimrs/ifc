//! Why an ifcXML operation failed.

use std::fmt;
use thiserror::Error;

/// Stable, codec-specific location of an ifcXML parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlPath(String);

impl XmlPath {
    pub(crate) fn new(path: String) -> Self {
        Self(path)
    }

    /// The XPath-like location string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for XmlPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Failures specific to reading or writing ifcXML.
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum XmlError {
    /// The document is not well-formed XML.
    #[error("malformed XML: {0}")]
    Malformed(String),
    /// An `id` attribute was not in the expected `i<number>` form.
    #[error("unparseable entity id {0:?}")]
    BadId(String),
    /// A typed scalar did not contain a value of its declared kind.
    #[error("invalid {kind} scalar {value:?}")]
    InvalidScalar {
        /// Declared scalar kind.
        kind: String,
        /// Invalid lexical value.
        value: String,
    },
    /// A non-empty explicit `kind` is not part of this lossless dialect.
    #[error("unknown value kind {0:?}")]
    UnknownKind(String),
    /// Two attributes or child elements of one entity name the same slot.
    #[error("`{name}` names slot {slot}, which is already set")]
    DuplicateSlot {
        /// The second name resolving to the slot.
        name: String,
        /// The zero-based positional slot.
        slot: usize,
    },
    /// A positional name skips slots that no schema declares as omittable.
    #[error("`{name}` names slot {slot}, but slot {missing} is absent")]
    MissingSlot {
        /// The name that skipped ahead.
        name: String,
        /// The zero-based slot it names.
        slot: usize,
        /// The first slot left without a value.
        missing: usize,
    },
    /// An element resolved outside the selected release namespace.
    #[error("element `{element}` has namespace {found:?}; expected `{expected}`")]
    Namespace {
        /// Element local name.
        element: String,
        /// Required namespace URI.
        expected: &'static str,
        /// Resolved namespace URI, if any.
        found: Option<String>,
    },
    /// Root schema metadata disagrees with the selected release profile.
    #[error("ifcXML profile expects schema `{expected}`, found {found:?}")]
    Profile {
        /// Required schema token.
        expected: &'static str,
        /// Declared root schema token, if any.
        found: Option<String>,
    },
    /// A strict document did not have the required root element.
    #[error("strict ifcXML requires root `ifcXML`, found {found:?}")]
    Root {
        /// First element local name, if any.
        found: Option<String>,
    },
    /// A parsing error with its entity/value location retained.
    #[error("{path}: {source}")]
    At {
        /// XPath-like parser location.
        path: XmlPath,
        /// Underlying typed parse error.
        #[source]
        source: Box<XmlError>,
    },
    /// Writing to the output buffer failed.
    #[error("write failed: {0}")]
    Write(String),
    /// An element names an entity type the schema does not declare.
    #[error("`{name}` is not an entity the schema declares")]
    UnknownEntity {
        /// The element or type name as written.
        name: String,
    },
    /// An abstract entity was instantiated.
    #[error("`{name}` is abstract and cannot be instantiated")]
    AbstractEntity {
        /// The declared entity name.
        name: String,
    },
    /// An XML attribute or child element that no attribute of the entity
    /// declares. Never placed into a free slot.
    #[error(
        "{entity} has no attribute `{attribute}` (on element `{element}`); \
         refusing to guess its slot"
    )]
    UnknownAttribute {
        /// The entity type, as the schema declares it.
        entity: String,
        /// The XML element carrying it: the entity's own element, which is
        /// an attribute name when the entity is defined in place.
        element: String,
        /// The XML attribute or child element name.
        attribute: String,
    },
    /// A value whose kind the declared type does not admit: an integer in a
    /// label, a reference where a measure is declared, an entity or typed
    /// value outside its SELECT.
    #[error("declared {declared}, found {found}")]
    TypeMismatch {
        /// The declared type, in EXPRESS terms.
        declared: String,
        /// What the document holds, in words.
        found: String,
    },
    /// An attribute written in a form its declared type does not take in
    /// this layout: a simple value as a child element, or an entity
    /// reference as an XML attribute.
    #[error("attribute `{attribute}` must be {expected}, found {found}")]
    WrongForm {
        /// The attribute name.
        attribute: String,
        /// The form the layout requires.
        expected: &'static str,
        /// The form the document used.
        found: &'static str,
    },
    /// A reference to an id no element of the document defines.
    #[error("reference to undefined id {id:?}")]
    UnresolvedReference {
        /// The referenced id as written.
        id: String,
    },
    /// Two elements define the same id.
    #[error("id {id:?} is defined twice")]
    DuplicateId {
        /// The repeated id.
        id: String,
    },
    /// An inverse attribute implies a value its relationship contradicts.
    #[error(
        "inverse `{inverse}` places this entity in `{relationship}.{attribute}`, \
         which {detail}"
    )]
    InverseConflict {
        /// The inverse attribute's name.
        inverse: String,
        /// The relationship entity's type.
        relationship: String,
        /// The relationship's explicit attribute the inverse inverts.
        attribute: String,
        /// Why the implied value cannot be placed.
        detail: String,
    },
    /// The schema given does not describe the selected release profile.
    #[error("ifcXML profile {profile} needs the {profile} schema, given `{schema}`")]
    SchemaMismatch {
        /// The profile's schema token.
        profile: &'static str,
        /// The given schema's name.
        schema: String,
    },
    /// Valid content this reader or writer does not implement. Refused,
    /// never approximated.
    #[error("unsupported: {construct}")]
    Unsupported {
        /// What was met, in words.
        construct: String,
    },
}

impl XmlError {
    /// Location retained by the reader, when the failure occurred in document content.
    #[must_use]
    pub fn path(&self) -> Option<&XmlPath> {
        match self {
            Self::At { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Innermost codec error, without discarding the inspectable path.
    #[must_use]
    pub fn root_cause(&self) -> &Self {
        match self {
            Self::At { source, .. } => source.root_cause(),
            error => error,
        }
    }

    pub(crate) fn at(self, path: String) -> Self {
        match self {
            Self::At { .. } => self,
            source => Self::At {
                path: XmlPath::new(path),
                source: Box::new(source),
            },
        }
    }
}

impl From<quick_xml::Error> for XmlError {
    fn from(error: quick_xml::Error) -> Self {
        Self::Malformed(error.to_string())
    }
}

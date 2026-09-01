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
#[derive(Debug, Error)]
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

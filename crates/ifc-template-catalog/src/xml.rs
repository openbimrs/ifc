//! PSD and QTO XML import.

mod common;
mod node;
mod property;
mod psd;
mod qto;

use thiserror::Error;

use crate::definition::{ApplicabilityError, SetTemplate};
use node::Node;

/// Resource limits for untrusted catalog XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportLimits {
    /// Maximum accepted document size in bytes.
    pub max_bytes: usize,
    /// Maximum number of parsed elements.
    pub max_nodes: usize,
    /// Maximum element nesting depth.
    pub max_depth: usize,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            max_bytes: 4 * 1024 * 1024,
            max_nodes: 100_000,
            max_depth: 128,
        }
    }
}

/// Parse one PSD `PropertySetDef` or QTO `QtoSetDef` document.
pub fn parse_template(xml: &str) -> Result<SetTemplate, XmlImportError> {
    parse_template_with_limits(xml, ImportLimits::default())
}

/// Parse one template while bounding input bytes, nodes, and nesting.
pub fn parse_template_with_limits(
    xml: &str,
    limits: ImportLimits,
) -> Result<SetTemplate, XmlImportError> {
    if xml.len() > limits.max_bytes {
        return Err(XmlImportError::LimitExceeded {
            kind: "bytes",
            limit: limits.max_bytes,
        });
    }
    let root = node::parse(xml, limits)?;
    match root.name.as_str() {
        "PropertySetDef" => psd::parse(&root),
        "QtoSetDef" => qto::parse(&root),
        value => Err(XmlImportError::UnsupportedRoot(value.to_owned())),
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
/// Why a PSD or QTO document could not be imported.
pub enum XmlImportError {
    #[error("XML {kind} limit exceeded ({limit})")]
    /// A declared resource limit was exceeded before parsing completed.
    LimitExceeded {
        /// Which limit was hit: bytes, nodes, or depth.
        kind: &'static str,
        /// The configured maximum for that limit.
        limit: usize,
    },
    #[error("XML has no root element")]
    /// The document contained no root element.
    MissingRoot,
    #[error("XML has multiple root elements")]
    /// The document declared more than one root element.
    MultipleRoots,
    #[error("invalid XML: {0}")]
    /// The document was not well-formed XML.
    Xml(String),
    #[error("unsupported catalog root `{0}`")]
    /// The root element is neither PropertySetDef nor QtoSetDef.
    UnsupportedRoot(String),
    #[error("missing `{field}` at `{path}`")]
    /// A required child element or attribute was absent.
    MissingField {
        /// Element path where the value was expected.
        path: String,
        /// Name of the absent field.
        field: String,
    },
    #[error("property `{path}` has zero or multiple property type elements")]
    /// A property declared zero or several property-type elements.
    AmbiguousPropertyType {
        /// Element path of the offending property.
        path: String,
    },
    #[error("unsupported property type `{element}` for `{set}.{property}`")]
    /// The property-type element is not one this importer maps.
    UnsupportedPropertyType {
        /// Name of the property set.
        set: String,
        /// Name of the property.
        property: String,
        /// The unmapped property-type element.
        element: String,
    },
    #[error("unsupported quantity type `{value}` for `{set}.{quantity}`")]
    /// The quantity-type token is not one this importer maps.
    UnsupportedQuantityType {
        /// Name of the quantity set.
        set: String,
        /// Name of the quantity.
        quantity: String,
        /// The unmapped quantity-type token.
        value: String,
    },
    #[error("unsupported property-set template type `{value}` for `{set}`")]
    /// The property-set template type token is not recognised.
    UnsupportedSetType {
        /// Name of the property set.
        set: String,
        /// The unrecognised template-type token.
        value: String,
    },
    #[error(transparent)]
    /// The parsed applicability expression was itself invalid.
    Applicability(#[from] ApplicabilityError),
}

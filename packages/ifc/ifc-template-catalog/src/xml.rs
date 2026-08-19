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
    pub max_bytes: usize,
    pub max_nodes: usize,
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
pub enum XmlImportError {
    #[error("XML {kind} limit exceeded ({limit})")]
    LimitExceeded { kind: &'static str, limit: usize },
    #[error("XML has no root element")]
    MissingRoot,
    #[error("XML has multiple root elements")]
    MultipleRoots,
    #[error("invalid XML: {0}")]
    Xml(String),
    #[error("unsupported catalog root `{0}`")]
    UnsupportedRoot(String),
    #[error("missing `{field}` at `{path}`")]
    MissingField { path: String, field: String },
    #[error("property `{path}` has zero or multiple property type elements")]
    AmbiguousPropertyType { path: String },
    #[error("unsupported property type `{element}` for `{set}.{property}`")]
    UnsupportedPropertyType {
        set: String,
        property: String,
        element: String,
    },
    #[error("unsupported quantity type `{value}` for `{set}.{quantity}`")]
    UnsupportedQuantityType {
        set: String,
        quantity: String,
        value: String,
    },
    #[error("unsupported property-set template type `{value}` for `{set}`")]
    UnsupportedSetType { set: String, value: String },
    #[error(transparent)]
    Applicability(#[from] ApplicabilityError),
}

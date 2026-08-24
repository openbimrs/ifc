//! Why an ifcXML operation failed.

use thiserror::Error;

/// Failures specific to reading or writing ifcXML.
#[derive(Debug, Error)]
pub enum XmlError {
    /// The document is not well-formed XML.
    #[error("malformed XML: {0}")]
    Malformed(String),
    /// An `id` attribute was not in the expected `i<number>` form.
    #[error("unparseable entity id {0:?}")]
    BadId(String),
    /// Writing to the output buffer failed.
    #[error("write failed: {0}")]
    Write(String),
}

impl From<quick_xml::Error> for XmlError {
    fn from(e: quick_xml::Error) -> Self {
        Self::Malformed(e.to_string())
    }
}

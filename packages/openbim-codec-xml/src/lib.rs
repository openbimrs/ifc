//! `openbim-codec-xml` — container-level XML recognition.
//!
//! # Why this is its own crate
//!
//! Both `packages/` (ifcXML) and `packages/` (BCF, IDS, IDM, LOIN,
//! ICDD) read XML. The workspace rule is that `packages/` must never depend
//! on `packages/`, so a shared XML layer cannot live inside `openbim`.
//! It sits below both instead.
//!
//! # Scope: recognition, not parsing
//!
//! This crate answers *"what is this byte stream, and where does its XML
//! start?"* — nothing more. Element trees, streaming readers and schema
//! binding belong to the format crates, which are free to use `quick-xml`
//! directly. Keeping the boundary here is what stops a shared "XML utilities"
//! crate from slowly absorbing every format's parsing quirks.
//!
//! # Detect by content, never by extension
//!
//! A file named `.bcf` may be a ZIP or a bare XML document depending on which
//! tool wrote it, and openBIM files are routinely misnamed in the wild.
//! Dispatching on the extension produces errors that read like corruption
//! rather than a wrong-container guess.
//!
//! # Status
//!
//! **Scaffold.** [`strip_bom`] and [`looks_like_xml`] are real and tested;
//! the crate is published so the layering is fixed before format code lands.

#![forbid(unsafe_code)]

/// UTF-8 byte-order mark.
///
/// Present in real openBIM files (several authoring tools emit it) and fatal
/// to a parser that expects a document to begin with `<`.
pub const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Whether the stream starts with a UTF-8 BOM.
#[must_use]
pub fn looks_like_bom(data: &[u8]) -> bool {
    data.starts_with(&UTF8_BOM)
}

/// Strip a leading UTF-8 BOM, returning the remaining bytes unchanged.
///
/// ```
/// use openbim_codec_xml::{strip_bom, UTF8_BOM};
/// let mut doc = UTF8_BOM.to_vec();
/// doc.extend_from_slice(b"<ids/>");
/// assert_eq!(strip_bom(&doc), b"<ids/>");
/// assert_eq!(strip_bom(b"<ids/>"), b"<ids/>");
/// ```
#[must_use]
pub fn strip_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(&UTF8_BOM[..]).unwrap_or(data)
}

/// Whether the stream plausibly begins an XML document.
///
/// Deliberately a *sniff*, not validation: it skips a BOM and leading
/// whitespace and checks for `<`. XML has no fixed magic number, so this can
/// only ever be a negative filter — it rules a stream out, it does not
/// certify one. Callers that need certainty must parse.
///
/// ```
/// use openbim_codec_xml::looks_like_xml;
/// assert!(looks_like_xml(b"  \n<?xml version=\"1.0\"?><ids/>"));
/// assert!(!looks_like_xml(b"PK\x03\x04"));
/// assert!(!looks_like_xml(b""));
/// ```
#[must_use]
pub fn looks_like_xml(data: &[u8]) -> bool {
    strip_bom(data)
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|&b| b == b'<')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_is_stripped_once() {
        let mut doc = UTF8_BOM.to_vec();
        doc.extend_from_slice(b"<a/>");
        assert!(looks_like_bom(&doc));
        assert_eq!(strip_bom(&doc), b"<a/>");
        // Idempotent on already-clean input.
        assert_eq!(strip_bom(strip_bom(&doc)), b"<a/>");
    }

    #[test]
    fn sniff_tolerates_bom_and_whitespace() {
        let mut doc = UTF8_BOM.to_vec();
        doc.extend_from_slice(b"\r\n\t <root/>");
        assert!(looks_like_xml(&doc));
    }

    #[test]
    fn sniff_rejects_zip_and_empty() {
        // A ZIP-wrapped BCF must not be handed to an XML reader.
        assert!(!looks_like_xml(b"PK\x03\x04"));
        assert!(!looks_like_xml(b""));
        assert!(!looks_like_xml(b"   "));
    }
}

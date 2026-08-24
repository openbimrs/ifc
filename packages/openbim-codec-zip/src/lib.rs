//! `openbim-codec-zip` — ZIP container recognition for openBIM archive formats.
//!
//! # Why this is its own crate
//!
//! Three unrelated things in this workspace ship as ZIP archives: BCF
//! (`.bcfzip`), ICDD (`.icdd`), and IFCZIP. They share the outer envelope and
//! nothing else. Recognising that envelope belongs in one place, below both
//! `packages/` and `packages/`, because IFCZIP is an IFC concern
//! and the other two are not.
//!
//! It is separate from `openbim-codec-xml` so that a consumer reading a plain `.ids`
//! file never links a ZIP implementation.
//!
//! # Scope: recognition, not extraction
//!
//! This crate identifies ZIP framing. Reading entries is the format crate's
//! job — BCF's per-topic directory layout and ICDD's `Index.rdf` conventions
//! have nothing in common beyond both being ZIPs, and a shared extraction
//! helper would have to encode one of them.
//!
//! # Status
//!
//! **Scaffold.** [`is_zip`] is real and tested; no entry reading yet.

#![forbid(unsafe_code)]

/// Local file header — a ZIP containing at least one entry.
pub const ZIP_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];
/// End-of-central-directory header — a valid but empty archive.
///
/// Worth matching explicitly: an empty `.bcfzip` is a real thing tools produce,
/// and rejecting it as "not a ZIP" misdiagnoses an empty issue list as
/// corruption.
pub const ZIP_EMPTY_MAGIC: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];
/// Spanned/split archive marker.
pub const ZIP_SPANNED_MAGIC: [u8; 4] = [0x50, 0x4B, 0x07, 0x08];

/// Whether the stream begins with any recognised ZIP framing.
///
/// Checks all three headers, not just the common one, so that empty and
/// spanned archives are identified as ZIPs rather than falling through to an
/// XML reader that will report a confusing parse error.
///
/// ```
/// use openbim_codec_zip::is_zip;
/// assert!(is_zip(b"PK\x03\x04rest"));
/// assert!(is_zip(b"PK\x05\x06"));      // valid, entry-less
/// assert!(!is_zip(b"<?xml version=\"1.0\"?>"));
/// ```
#[must_use]
pub fn is_zip(data: &[u8]) -> bool {
    let Some(head) = data.get(..4) else {
        return false;
    };
    head == ZIP_MAGIC || head == ZIP_EMPTY_MAGIC || head == ZIP_SPANNED_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_all_three_headers() {
        assert!(is_zip(b"PK\x03\x04payload"));
        assert!(is_zip(b"PK\x05\x06"));
        assert!(is_zip(b"PK\x07\x08"));
    }

    #[test]
    fn rejects_xml_and_short_input() {
        assert!(!is_zip(b"<?xml version=\"1.0\"?>"));
        assert!(!is_zip(b"PK\x03"));
        assert!(!is_zip(b""));
    }
}

//! Versioned binary artifact codec.

use bincode::{Decode, Encode};
use thiserror::Error;

use crate::catalog::{Catalog, CatalogError, CatalogProfile};
use crate::definition::{SetTemplate, SourceManifest};

const MAGIC: [u8; 8] = *b"NEHPSDQ\0";
const FORMAT_VERSION: u16 = 2;
const MIN_HEADER_BYTES: usize = MAGIC.len() + 1;
const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Encode, Decode)]
struct ArchivePayload {
    manifest: SourceManifest,
    templates: Vec<SetTemplate>,
}

/// Decode a versioned binary catalog artifact into an official-profile [`Catalog`].
///
/// Fails on oversized input, bad magic, a truncated or unsupported-version
/// header, undecodable payload bytes, trailing bytes after the payload, or a
/// payload that fails [`Catalog::try_new`] (duplicate/empty names, manifest
/// count mismatch).
pub fn decode_catalog(bytes: &[u8]) -> Result<Catalog, ArchiveError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::TooLarge {
            actual: bytes.len(),
            limit: MAX_ARCHIVE_BYTES,
        });
    }
    if !bytes.starts_with(&MAGIC) {
        return Err(ArchiveError::BadMagic);
    }
    if bytes.len() < MIN_HEADER_BYTES {
        return Err(ArchiveError::TruncatedHeader {
            actual: bytes.len(),
            required: MIN_HEADER_BYTES,
        });
    }
    let header_config = bincode::config::standard().with_limit::<16>();
    let (format_version, version_bytes): (u16, usize) =
        bincode::decode_from_slice(&bytes[MAGIC.len()..], header_config)
            .map_err(|error| ArchiveError::Decode(error.to_string()))?;
    if format_version != FORMAT_VERSION {
        return Err(ArchiveError::UnsupportedVersion(format_version));
    }
    let payload_bytes = &bytes[MAGIC.len() + version_bytes..];
    let config = bincode::config::standard().with_limit::<MAX_ARCHIVE_BYTES>();
    let (archive, consumed): (ArchivePayload, usize) =
        bincode::decode_from_slice(payload_bytes, config)
            .map_err(|error| ArchiveError::Decode(error.to_string()))?;
    if consumed != payload_bytes.len() {
        return Err(ArchiveError::TrailingBytes(payload_bytes.len() - consumed));
    }
    Catalog::try_new(
        archive.manifest,
        CatalogProfile::Official,
        archive.templates,
    )
    .map_err(ArchiveError::Catalog)
}

/// Encode a manifest and template list into the versioned binary artifact
/// format that [`decode_catalog`] reads back.
#[cfg(feature = "generation")]
pub fn encode_catalog(
    manifest: SourceManifest,
    templates: Vec<SetTemplate>,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    let archive = ArchivePayload {
        manifest,
        templates,
    };
    let payload = bincode::encode_to_vec(archive, bincode::config::standard())?;
    let version = bincode::encode_to_vec(FORMAT_VERSION, bincode::config::standard())?;
    let mut bytes = Vec::with_capacity(MAGIC.len() + version.len() + payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&version);
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
/// Why a serialized catalog artifact could not be decoded.
pub enum ArchiveError {
    /// The bincode payload could not be decoded; message carries the underlying error.
    #[error("cannot decode catalog artifact: {0}")]
    Decode(String),
    /// Input exceeds the 8 MiB resource budget for a catalog artifact.
    #[error("catalog artifact is {actual} bytes; limit is {limit} bytes")]
    TooLarge {
        /// Actual input length in bytes.
        actual: usize,
        /// Maximum permitted length in bytes.
        limit: usize,
    },
    /// Input is shorter than the magic plus minimum version header.
    #[error("catalog artifact header is {actual} bytes; at least {required} bytes are required")]
    TruncatedHeader {
        /// Actual input length in bytes.
        actual: usize,
        /// Minimum header length required.
        required: usize,
    },
    /// Input does not start with the expected 8-byte magic sequence.
    #[error("catalog artifact magic is invalid")]
    BadMagic,
    /// The decoded format-version number is not the version this build reads.
    #[error("unsupported catalog artifact format version {0}")]
    UnsupportedVersion(u16),
    /// Bytes remained after the payload was fully decoded.
    #[error("catalog artifact has {0} trailing bytes")]
    TrailingBytes(usize),
    /// The decoded payload failed [`Catalog::try_new`] validation.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
}

#[cfg(test)]
mod tests {
    use super::{decode_catalog, ArchiveError, MAX_ARCHIVE_BYTES};

    #[test]
    fn reports_archive_version_before_decoding_its_payload() {
        let mut bytes = super::MAGIC.to_vec();
        bytes.push(1); // bincode's legacy format-version encoding
        bytes.push(1); // first payload byte must not be consumed as header
        assert!(matches!(
            decode_catalog(&bytes),
            Err(ArchiveError::UnsupportedVersion(1))
        ));
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(matches!(
            decode_catalog(&super::MAGIC),
            Err(ArchiveError::TruncatedHeader { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = include_bytes!("../data/ifc4-add2-tc1.bin").to_vec();
        bytes.push(0);
        assert!(matches!(
            decode_catalog(&bytes),
            Err(ArchiveError::TrailingBytes(1))
        ));
    }

    #[test]
    fn decode_rejects_input_above_resource_budget() {
        let bytes = vec![0; MAX_ARCHIVE_BYTES + 1];
        assert!(matches!(
            decode_catalog(&bytes),
            Err(ArchiveError::TooLarge { .. })
        ));
    }
}

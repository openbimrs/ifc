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
pub enum ArchiveError {
    #[error("cannot decode catalog artifact: {0}")]
    Decode(String),
    #[error("catalog artifact is {actual} bytes; limit is {limit} bytes")]
    TooLarge { actual: usize, limit: usize },
    #[error("catalog artifact header is {actual} bytes; at least {required} bytes are required")]
    TruncatedHeader { actual: usize, required: usize },
    #[error("catalog artifact magic is invalid")]
    BadMagic,
    #[error("unsupported catalog artifact format version {0}")]
    UnsupportedVersion(u16),
    #[error("catalog artifact has {0} trailing bytes")]
    TrailingBytes(usize),
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

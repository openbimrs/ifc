//! Versioned binary artifact codec.

use bincode::{Decode, Encode};
use thiserror::Error;

use crate::catalog::{Catalog, CatalogError, CatalogProfile};
use crate::definition::{SetTemplate, SourceManifest};

const MAGIC: [u8; 8] = *b"NEHPSDQ\0";
const FORMAT_VERSION: u16 = 1;
const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Encode, Decode)]
struct Archive {
    magic: [u8; 8],
    format_version: u16,
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
    let config = bincode::config::standard().with_limit::<MAX_ARCHIVE_BYTES>();
    let (archive, consumed): (Archive, usize) = bincode::decode_from_slice(bytes, config)
        .map_err(|error| ArchiveError::Decode(error.to_string()))?;
    if consumed != bytes.len() {
        return Err(ArchiveError::TrailingBytes(bytes.len() - consumed));
    }
    if archive.magic != MAGIC {
        return Err(ArchiveError::BadMagic);
    }
    if archive.format_version != FORMAT_VERSION {
        return Err(ArchiveError::UnsupportedVersion(archive.format_version));
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
    let archive = Archive {
        magic: MAGIC,
        format_version: FORMAT_VERSION,
        manifest,
        templates,
    };
    bincode::encode_to_vec(archive, bincode::config::standard())
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ArchiveError {
    #[error("cannot decode catalog artifact: {0}")]
    Decode(String),
    #[error("catalog artifact is {actual} bytes; limit is {limit} bytes")]
    TooLarge { actual: usize, limit: usize },
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
    fn decode_rejects_input_above_resource_budget() {
        let bytes = vec![0; MAX_ARCHIVE_BYTES + 1];
        assert!(matches!(
            decode_catalog(&bytes),
            Err(ArchiveError::TooLarge { .. })
        ));
    }
}

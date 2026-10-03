//! The compact snapshot format (version 3) and its pinned files.
//!
//! One format serves two shapes:
//!
//! - the **container** this crate embeds (`data/catalog.bin`, feature
//!   `embedded`) holds every edition over one shared string table and one
//!   set of shared records, so text and templates two editions publish
//!   identically are stored once;
//! - a **per-edition file** ([`file_name`]) holds one edition. Hosts that
//!   load a catalog at runtime (the npm package, feature `runtime`) ship
//!   these, so a reader pays only for the edition it uses.
//!
//! Both are lossless format shifts of the authenticated buildingSMART
//! PSD/QTO XML (ADR 0017): every set, member, definition, translation,
//! applicability selector and provenance field decodes exactly as imported.
//! The encoder is deterministic, and each file's SHA-256 is pinned here
//! ([`pinned_sha256`], [`CONTAINER_SHA256`]); `runtime` refuses bytes that do
//! not match their pin.

use thiserror::Error;

use crate::catalog::{Catalog, CatalogError, CatalogProfile};
use crate::definition::CatalogEdition;
use crate::pack::{self, PackError};

/// The snapshot format version this build reads and writes.
pub const FORMAT_VERSION: u16 = 3;

/// SHA-256 of the embedded container, `data/catalog.bin`, holding every
/// edition.
pub const CONTAINER_SHA256: &str =
    "875a3902ac9a5c9d1f3b21dcbd5afe27fc65d1363be4452e852f9c6815c75996";

/// Every edition with a committed snapshot, in container order.
pub const EDITIONS: [CatalogEdition; 3] = [
    CatalogEdition::Ifc2x3Tc1,
    CatalogEdition::Ifc4Add2Tc1,
    CatalogEdition::Ifc4x3Add2,
];

/// The conventional file name of `edition`'s per-edition snapshot, such as
/// `ifc4x3-add2.bin`.
pub fn file_name(edition: CatalogEdition) -> &'static str {
    match edition {
        CatalogEdition::Ifc2x3Tc1 => "ifc2x3-tc1.bin",
        CatalogEdition::Ifc4Add2Tc1 => "ifc4-add2-tc1.bin",
        CatalogEdition::Ifc4x3Add2 => "ifc4x3-add2.bin",
    }
}

/// SHA-256 (lowercase hexadecimal) of `edition`'s per-edition snapshot, as
/// [`encode`] writes it from the committed container.
pub fn pinned_sha256(edition: CatalogEdition) -> &'static str {
    match edition {
        CatalogEdition::Ifc2x3Tc1 => {
            "b416694e2ddcb0d3011a7acb1b23def34ae16b3f2d27156be74c594d42248955"
        }
        CatalogEdition::Ifc4Add2Tc1 => {
            "1ea624205c45cb31489f265c53fcef1334ba8556046cf296f73ee4e7550c6539"
        }
        CatalogEdition::Ifc4x3Add2 => {
            "0cfa3e0700b98e65ad1c2743e92f65299246a4aaf663520837346196179a29c2"
        }
    }
}

/// The editions a snapshot holds, in its order, without decoding their
/// templates.
pub fn editions(bytes: &[u8]) -> Result<Vec<CatalogEdition>, ArchiveError> {
    check_header(bytes)?;
    pack::editions(bytes).map_err(ArchiveError::from)
}

/// Decode `edition` from a container or a per-edition file into an
/// official-profile [`Catalog`]. Only that edition's records are decoded.
///
/// No digest is checked here; `runtime::install` checks the pin first.
pub fn decode_edition(bytes: &[u8], edition: CatalogEdition) -> Result<Catalog, ArchiveError> {
    check_header(bytes)?;
    pack::decode_one(bytes, edition)?.ok_or(ArchiveError::MissingEdition(edition))
}

/// Decode every edition of a snapshot into official-profile catalogs, in
/// its order.
pub fn decode_all(bytes: &[u8]) -> Result<Vec<Catalog>, ArchiveError> {
    check_header(bytes)?;
    pack::decode_all(bytes).map_err(ArchiveError::from)
}

/// Encode official catalogs into one snapshot, in edition order whatever
/// the order given: one catalog makes a per-edition file, every edition the
/// container. Equal catalogs always encode to equal bytes.
pub fn encode(catalogs: &[&Catalog]) -> Result<Vec<u8>, EncodeError> {
    let mut sorted: Vec<&Catalog> = catalogs.to_vec();
    sorted.sort_by_key(|catalog| catalog.manifest().edition);
    for pair in sorted.windows(2) {
        if pair[0].manifest().edition == pair[1].manifest().edition {
            return Err(EncodeError::DuplicateEdition(pair[0].manifest().edition));
        }
    }
    if let Some(catalog) = sorted
        .iter()
        .find(|catalog| catalog.profile() != CatalogProfile::Official)
    {
        return Err(EncodeError::NotOfficial(catalog.profile()));
    }
    let templates: Vec<Vec<_>> = sorted
        .iter()
        .map(|catalog| catalog.iter().cloned().collect())
        .collect();
    let editions: Vec<_> = sorted
        .iter()
        .zip(&templates)
        .map(|(catalog, templates)| (catalog.manifest(), templates.as_slice()))
        .collect();
    Ok(pack::encode(&editions))
}

fn check_header(bytes: &[u8]) -> Result<(), ArchiveError> {
    if bytes.len() > pack::MAX_BYTES {
        return Err(ArchiveError::TooLarge {
            actual: bytes.len(),
            limit: pack::MAX_BYTES,
        });
    }
    if !bytes.starts_with(&pack::MAGIC) {
        return Err(ArchiveError::BadMagic);
    }
    if bytes.len() < MIN_HEADER_BYTES {
        return Err(ArchiveError::TruncatedHeader {
            actual: bytes.len(),
            required: MIN_HEADER_BYTES,
        });
    }
    Ok(())
}

const MIN_HEADER_BYTES: usize = pack::MAGIC.len() + 1;

/// Why a snapshot cannot be encoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EncodeError {
    /// Only official catalogs are snapshots; a corrected or custom profile
    /// is derived from one at load time.
    #[error("only official catalogs are encoded, not {0:?}")]
    NotOfficial(CatalogProfile),
    /// Two catalogs of one edition were given.
    #[error("edition {0:?} is given twice")]
    DuplicateEdition(CatalogEdition),
}

/// Why a serialized catalog snapshot could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ArchiveError {
    /// The payload is malformed; the message names what is wrong.
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
    /// The snapshot is well formed but holds no such edition.
    #[error("the catalog artifact holds no {0:?} edition")]
    MissingEdition(CatalogEdition),
}

impl From<PackError> for ArchiveError {
    fn from(error: PackError) -> Self {
        match error {
            PackError::TooLarge(actual) => Self::TooLarge {
                actual,
                limit: pack::MAX_BYTES,
            },
            PackError::BadMagic => Self::BadMagic,
            PackError::UnsupportedVersion(version) => match u16::try_from(version) {
                Ok(version) => Self::UnsupportedVersion(version),
                Err(_) => Self::Decode(format!("format version {version} is out of range")),
            },
            PackError::Malformed(what) => Self::Decode(what.to_owned()),
            PackError::TrailingBytes(count) => Self::TrailingBytes(count),
            PackError::Catalog(error) => Self::Catalog(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_all, decode_edition, ArchiveError};
    use crate::definition::CatalogEdition;
    use crate::pack::{MAGIC, MAX_BYTES};

    fn container() -> &'static [u8] {
        include_bytes!("../data/catalog.bin")
    }

    #[test]
    fn reports_format_version_before_decoding_its_payload() {
        let mut bytes = MAGIC.to_vec();
        bytes.push(2); // the bincode format this one replaces
        bytes.push(1); // first payload byte must not be consumed as header
        assert_eq!(
            decode_all(&bytes).unwrap_err(),
            ArchiveError::UnsupportedVersion(2)
        );
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(matches!(
            decode_all(&MAGIC),
            Err(ArchiveError::TruncatedHeader { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = container().to_vec();
        bytes.push(0);
        assert!(matches!(
            decode_edition(&bytes, CatalogEdition::Ifc4Add2Tc1),
            Err(ArchiveError::TrailingBytes(1))
        ));
    }

    #[test]
    fn rejects_every_truncation_of_a_small_snapshot() {
        let catalog = decode_edition(container(), CatalogEdition::Ifc2x3Tc1).unwrap();
        let bytes = super::encode(&[&catalog]).unwrap();
        // Cutting anywhere is refused, never decoded short or panicking.
        for len in (0..bytes.len()).step_by(997) {
            assert!(decode_all(&bytes[..len]).is_err(), "cut at {len}");
        }
    }

    #[test]
    fn rejects_a_flipped_record_byte_without_panicking() {
        let catalog = decode_edition(container(), CatalogEdition::Ifc2x3Tc1).unwrap();
        let bytes = super::encode(&[&catalog]).unwrap();
        for at in (MAGIC.len()..bytes.len()).step_by(4099) {
            let mut corrupt = bytes.clone();
            corrupt[at] ^= 0xff;
            // Either refused or decoded; the point is no panic and no hang.
            let _ = decode_all(&corrupt);
        }
    }

    #[test]
    fn decode_rejects_input_above_resource_budget() {
        let bytes = vec![0; MAX_BYTES + 1];
        assert!(matches!(
            decode_all(&bytes),
            Err(ArchiveError::TooLarge { .. })
        ));
    }

    #[test]
    fn a_missing_edition_is_named() {
        let catalog = decode_edition(container(), CatalogEdition::Ifc2x3Tc1).unwrap();
        let bytes = super::encode(&[&catalog]).unwrap();
        assert_eq!(
            decode_edition(&bytes, CatalogEdition::Ifc4x3Add2).unwrap_err(),
            ArchiveError::MissingEdition(CatalogEdition::Ifc4x3Add2)
        );
    }
}

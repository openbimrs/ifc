//! Catalogs supplied at runtime, checked against their pinned digests.
//!
//! A host that leaves the catalog out of its binary (the npm package does,
//! to keep 1.4 MB out of every browser download) ships the per-edition
//! snapshot files instead and hands their bytes to [`install`]. The bytes
//! must hash to the edition's pin ([`snapshot::pinned_sha256`]) or to the
//! container's ([`snapshot::CONTAINER_SHA256`]); anything else is refused, so
//! a runtime catalog is the same data the embedded one is.
//!
//! Installed catalogs live for the process (for a WebAssembly module, for
//! the module instance). [`load_catalog`] reads them like
//! [`crate::embedded::load_catalog`] reads the embedded ones, in the
//! official or the corrected profile, and refuses an edition not installed
//! yet with [`RuntimeCatalogError::NotInstalled`].
//!
//! [`snapshot::pinned_sha256`]: crate::snapshot::pinned_sha256
//! [`snapshot::CONTAINER_SHA256`]: crate::snapshot::CONTAINER_SHA256

use std::sync::OnceLock;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::catalog::{Catalog, CatalogProfile};
use crate::definition::CatalogEdition;
use crate::overlay::{corrected_patches, PatchError};
use crate::snapshot::{self, ArchiveError};

static IFC2X3_TC1: OnceLock<Catalog> = OnceLock::new();
static IFC4_ADD2_TC1: OnceLock<Catalog> = OnceLock::new();
static IFC4X3_ADD2: OnceLock<Catalog> = OnceLock::new();
static IFC4_ADD2_TC1_CORRECTED: OnceLock<Result<Catalog, PatchError>> = OnceLock::new();
static IFC4X3_ADD2_CORRECTED: OnceLock<Result<Catalog, PatchError>> = OnceLock::new();

fn official_slot(edition: CatalogEdition) -> &'static OnceLock<Catalog> {
    match edition {
        CatalogEdition::Ifc2x3Tc1 => &IFC2X3_TC1,
        CatalogEdition::Ifc4Add2Tc1 => &IFC4_ADD2_TC1,
        CatalogEdition::Ifc4x3Add2 => &IFC4X3_ADD2,
    }
}

/// Check `bytes` against `edition`'s pinned SHA-256, or the container's,
/// and decode that edition. Nothing is installed.
pub fn decode_verified(
    edition: CatalogEdition,
    bytes: &[u8],
) -> Result<Catalog, RuntimeCatalogError> {
    verify(edition, bytes)?;
    Ok(snapshot::decode_edition(bytes, edition)?)
}

/// Install `edition` from snapshot bytes: its per-edition file, or the
/// container. The bytes are checked against the pin before anything is
/// decoded. Installing an edition already installed is a no-op that still
/// checks the bytes.
pub fn install(edition: CatalogEdition, bytes: &[u8]) -> Result<(), RuntimeCatalogError> {
    let slot = official_slot(edition);
    if slot.get().is_some() {
        // Still refuse bytes that are not the pinned snapshot.
        verify(edition, bytes)?;
        return Ok(());
    }
    let catalog = decode_verified(edition, bytes)?;
    // A concurrent install of the same pinned bytes may have won; it
    // installed the same catalog.
    let _ = slot.set(catalog);
    Ok(())
}

fn verify(edition: CatalogEdition, bytes: &[u8]) -> Result<(), RuntimeCatalogError> {
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let expected = snapshot::pinned_sha256(edition);
    if actual == expected || actual == snapshot::CONTAINER_SHA256 {
        Ok(())
    } else {
        Err(RuntimeCatalogError::DigestMismatch {
            edition,
            expected,
            actual,
        })
    }
}

/// Whether `edition` has been installed.
pub fn is_installed(edition: CatalogEdition) -> bool {
    official_slot(edition).get().is_some()
}

/// An installed catalog in `profile`: [`CatalogProfile::Official`], or
/// [`CatalogProfile::Corrected`] for the editions with a built-in overlay
/// (IFC4 ADD2 TC1, IFC4X3 ADD2), derived once and cached.
pub fn load_catalog(
    edition: CatalogEdition,
    profile: CatalogProfile,
) -> Result<Catalog, RuntimeCatalogError> {
    let official = official_slot(edition)
        .get()
        .cloned()
        .ok_or(RuntimeCatalogError::NotInstalled(edition))?;
    match profile {
        CatalogProfile::Official => Ok(official),
        CatalogProfile::Corrected => {
            let slot = match edition {
                CatalogEdition::Ifc4Add2Tc1 => &IFC4_ADD2_TC1_CORRECTED,
                CatalogEdition::Ifc4x3Add2 => &IFC4X3_ADD2_CORRECTED,
                _ => return Err(RuntimeCatalogError::UnavailableEdition(edition)),
            };
            slot.get_or_init(|| {
                official.with_patches(CatalogProfile::Corrected, &corrected_patches(edition))
            })
            .clone()
            .map_err(RuntimeCatalogError::Patch)
        }
        _ => Err(RuntimeCatalogError::UnsupportedProfile(profile)),
    }
}

/// Why a runtime catalog cannot be installed or read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RuntimeCatalogError {
    /// The bytes are not the pinned snapshot of this edition or the pinned
    /// container: a different edition's file, another version, or a
    /// damaged download.
    #[error(
        "the {edition:?} catalog bytes have SHA-256 {actual}; the pinned snapshot has {expected}"
    )]
    DigestMismatch {
        /// The edition the bytes were offered for.
        edition: CatalogEdition,
        /// The edition's pinned per-edition SHA-256.
        expected: &'static str,
        /// The offered bytes' SHA-256.
        actual: String,
    },
    /// The pinned bytes failed to decode (a build whose decoder disagrees
    /// with its own pins).
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// No catalog of this edition has been installed yet.
    #[error("no {0:?} catalog is installed; install its snapshot first")]
    NotInstalled(CatalogEdition),
    /// The edition has no built-in corrected profile (IFC2X3 TC1).
    #[error("no corrected profile for {0:?}")]
    UnavailableEdition(CatalogEdition),
    /// Only the official and corrected profiles are loaded this way.
    #[error("runtime loading does not construct {0:?} profiles")]
    UnsupportedProfile(CatalogProfile),
    /// Applying the built-in correction ledger failed.
    #[error(transparent)]
    Patch(#[from] PatchError),
}

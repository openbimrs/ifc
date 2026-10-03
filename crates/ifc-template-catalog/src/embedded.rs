//! Embedded official and corrected catalog snapshots.
//!
//! Every edition comes from one container, `data/catalog.bin`
//! ([`crate::snapshot`]), which stores text and templates the editions share
//! once. The first load of an edition decodes that edition's records only.

use std::sync::OnceLock;

use thiserror::Error;

use crate::catalog::{Catalog, CatalogProfile};
use crate::definition::CatalogEdition;
use crate::overlay::{corrected_patches, PatchError};
use crate::snapshot::{decode_edition, ArchiveError};

/// The committed container holding every edition.
pub(crate) const CONTAINER: &[u8] = include_bytes!("../data/catalog.bin");

static IFC2X3_TC1: OnceLock<Result<Catalog, ArchiveError>> = OnceLock::new();
static IFC4_ADD2_TC1: OnceLock<Result<Catalog, ArchiveError>> = OnceLock::new();
static IFC4X3_ADD2: OnceLock<Result<Catalog, ArchiveError>> = OnceLock::new();
static IFC4_ADD2_TC1_CORRECTED: OnceLock<Result<Catalog, EmbeddedCatalogError>> = OnceLock::new();
static IFC4X3_ADD2_CORRECTED: OnceLock<Result<Catalog, EmbeddedCatalogError>> = OnceLock::new();

/// Load a catalog snapshot from committed generated data.
pub fn load_catalog(
    edition: CatalogEdition,
    profile: CatalogProfile,
) -> Result<Catalog, EmbeddedCatalogError> {
    match profile {
        CatalogProfile::Official => official_catalog(edition),
        CatalogProfile::Corrected => corrected_catalog(edition),
        _ => Err(EmbeddedCatalogError::UnsupportedProfile(profile)),
    }
}

/// Load an unmodified official catalog.
pub fn official_catalog(edition: CatalogEdition) -> Result<Catalog, EmbeddedCatalogError> {
    let slot = match edition {
        CatalogEdition::Ifc2x3Tc1 => &IFC2X3_TC1,
        CatalogEdition::Ifc4Add2Tc1 => &IFC4_ADD2_TC1,
        CatalogEdition::Ifc4x3Add2 => &IFC4X3_ADD2,
    };
    slot.get_or_init(|| decode_edition(CONTAINER, edition))
        .clone()
        .map_err(EmbeddedCatalogError::Archive)
}

/// Load the official catalog with the ordered built-in correction ledger.
pub fn corrected_catalog(edition: CatalogEdition) -> Result<Catalog, EmbeddedCatalogError> {
    let slot = match edition {
        CatalogEdition::Ifc4Add2Tc1 => &IFC4_ADD2_TC1_CORRECTED,
        CatalogEdition::Ifc4x3Add2 => &IFC4X3_ADD2_CORRECTED,
        _ => return Err(EmbeddedCatalogError::UnavailableEdition(edition)),
    };
    slot.get_or_init(|| {
        let official = official_catalog(edition)?;
        official
            .with_patches(CatalogProfile::Corrected, &corrected_patches(edition))
            .map_err(EmbeddedCatalogError::Patch)
    })
    .clone()
}

/// Why loading a committed embedded catalog snapshot failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EmbeddedCatalogError {
    /// No committed generated data exists for this edition/profile pair.
    #[error("no embedded catalog for {0:?}")]
    UnavailableEdition(CatalogEdition),
    /// [`load_catalog`] was called with a profile other than `Official` or `Corrected`.
    #[error("embedded loading does not construct {0:?} profiles")]
    UnsupportedProfile(CatalogProfile),
    /// The committed binary artifact failed to decode.
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// Applying the built-in correction ledger to the official catalog failed.
    #[error(transparent)]
    Patch(#[from] PatchError),
}

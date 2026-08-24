//! Embedded official and corrected catalog snapshots.

use std::sync::OnceLock;

use thiserror::Error;

use crate::archive::{decode_catalog, ArchiveError};
use crate::catalog::{Catalog, CatalogProfile};
use crate::definition::CatalogEdition;
use crate::overlay::{corrected_patches, PatchError};

static IFC4_ADD2_TC1: OnceLock<Result<Catalog, ArchiveError>> = OnceLock::new();
static IFC4_ADD2_TC1_CORRECTED: OnceLock<Result<Catalog, EmbeddedCatalogError>> = OnceLock::new();

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
    match edition {
        CatalogEdition::Ifc4Add2Tc1 => IFC4_ADD2_TC1
            .get_or_init(|| decode_catalog(include_bytes!("../data/ifc4-add2-tc1.bin")))
            .clone()
            .map_err(EmbeddedCatalogError::Archive),
        _ => Err(EmbeddedCatalogError::UnavailableEdition(edition)),
    }
}

/// Load the official catalog with the ordered built-in correction ledger.
pub fn corrected_catalog(edition: CatalogEdition) -> Result<Catalog, EmbeddedCatalogError> {
    match edition {
        CatalogEdition::Ifc4Add2Tc1 => IFC4_ADD2_TC1_CORRECTED
            .get_or_init(|| {
                let official = official_catalog(edition)?;
                official
                    .with_patches(CatalogProfile::Corrected, &corrected_patches(edition))
                    .map_err(EmbeddedCatalogError::Patch)
            })
            .clone(),
        _ => Err(EmbeddedCatalogError::UnavailableEdition(edition)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum EmbeddedCatalogError {
    #[error("no embedded catalog for {0:?}")]
    UnavailableEdition(CatalogEdition),
    #[error("embedded loading does not construct {0:?} profiles")]
    UnsupportedProfile(CatalogProfile),
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    Patch(#[from] PatchError),
}

use ifc_schema::SchemaVersion;
use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
use ifc_template_catalog::definition::CatalogEdition;

/// Why [`release_catalog`] returned no catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReleaseCatalogError {
    /// The release has no catalog (IFC4X1, IFC4X2), or is one this build
    /// does not know. No other release's catalog is read in its place.
    NoCatalog(SchemaVersion),
    /// A runtime-catalog build before the release's edition is installed.
    NotInstalled {
        /// The edition to install first.
        edition: CatalogEdition,
    },
    /// The catalog exists but cannot be read; the text is the reason.
    Unreadable(String),
}

impl std::fmt::Display for ReleaseCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCatalog(version) => write!(f, "{version:?} has no PSD/QTO catalog"),
            Self::NotInstalled { edition } => write!(
                f,
                "no {edition:?} catalog is installed; install its snapshot first"
            ),
            Self::Unreadable(reason) => write!(f, "the catalog cannot be read: {reason}"),
        }
    }
}

impl std::error::Error for ReleaseCatalogError {}

/// The PSD/QTO catalog `version` reads: the edition and profile property
/// edits and validation use for it. The returned [`Catalog`] shares its
/// data (it is reference-counted), so calling this repeatedly is cheap.
///
/// # Errors
///
/// [`ReleaseCatalogError::NoCatalog`] for a release without one,
/// [`ReleaseCatalogError::NotInstalled`] in a `property-catalog-runtime`
/// build (without `property-catalog`) before its edition is installed, and
/// [`ReleaseCatalogError::Unreadable`] if the catalog cannot be read.
pub fn release_catalog(version: SchemaVersion) -> Result<Catalog, ReleaseCatalogError> {
    let (edition, profile) = match version {
        SchemaVersion::Ifc2x3 => (CatalogEdition::Ifc2x3Tc1, CatalogProfile::Official),
        SchemaVersion::Ifc4 => (CatalogEdition::Ifc4Add2Tc1, CatalogProfile::Corrected),
        SchemaVersion::Ifc4x3 => (CatalogEdition::Ifc4x3Add2, CatalogProfile::Corrected),
        other => return Err(ReleaseCatalogError::NoCatalog(other)),
    };
    #[cfg(feature = "property-catalog")]
    {
        ifc_template_catalog::embedded::load_catalog(edition, profile)
            .map_err(|error| ReleaseCatalogError::Unreadable(error.to_string()))
    }
    #[cfg(not(feature = "property-catalog"))]
    {
        use ifc_template_catalog::runtime::{load_catalog, RuntimeCatalogError};
        load_catalog(edition, profile).map_err(|error| match error {
            RuntimeCatalogError::NotInstalled(edition) => {
                ReleaseCatalogError::NotInstalled { edition }
            }
            other => ReleaseCatalogError::Unreadable(other.to_string()),
        })
    }
}

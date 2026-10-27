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
    /// The catalog exists but cannot be read.
    Unreadable {
        /// The edition that failed to read.
        edition: CatalogEdition,
        /// Why, as the catalog crate reports it.
        reason: String,
    },
}

impl std::fmt::Display for ReleaseCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCatalog(version) => write!(f, "{version:?} has no PSD/QTO catalog"),
            Self::NotInstalled { edition } => write!(
                f,
                "no {edition:?} catalog is installed; install its snapshot first"
            ),
            Self::Unreadable { edition, reason } => {
                write!(f, "the {edition:?} catalog cannot be read: {reason}")
            }
        }
    }
}

impl std::error::Error for ReleaseCatalogError {}

/// The catalog edition `version` reads, or `None` for a release without
/// one (IFC4X1, IFC4X2, or a release this build does not know): IFC2X3 reads
/// IFC2X3 TC1, IFC4 reads IFC4 ADD2 TC1, IFC4X3 reads IFC4X3 ADD2. The one
/// copy of this table; [`release_catalog`] and the bindings use it.
#[must_use]
pub fn release_edition(version: SchemaVersion) -> Option<CatalogEdition> {
    match version {
        SchemaVersion::Ifc2x3 => Some(CatalogEdition::Ifc2x3Tc1),
        SchemaVersion::Ifc4 => Some(CatalogEdition::Ifc4Add2Tc1),
        SchemaVersion::Ifc4x3 => Some(CatalogEdition::Ifc4x3Add2),
        _ => None,
    }
}

/// The PSD/QTO catalog `version` reads: the edition and profile property
/// edits and validation use for it, so an authoring check made before a
/// product exists cannot disagree with them.
///
/// | Release | Edition ([`release_edition`]) | Profile |
/// | --- | --- | --- |
/// | IFC2X3 | IFC2X3 TC1 | as published (`Official`) |
/// | IFC4 | IFC4 ADD2 TC1 | built-in corrections (`Corrected`) |
/// | IFC4X1, IFC4X2 | none | [`ReleaseCatalogError::NoCatalog`] |
/// | IFC4X3 | IFC4X3 ADD2 | built-in corrections (`Corrected`) |
///
/// With `property-catalog` the catalog is the embedded one; with only
/// `property-catalog-runtime` it is the one the host installed. The returned
/// [`Catalog`] shares its data (it is reference-counted), so calling this
/// repeatedly is cheap.
///
/// # Errors
///
/// [`ReleaseCatalogError::NoCatalog`] for a release without one,
/// [`ReleaseCatalogError::NotInstalled`] in a `property-catalog-runtime`
/// build (without `property-catalog`) before its edition is installed, and
/// [`ReleaseCatalogError::Unreadable`] if the catalog cannot be read.
pub fn release_catalog(version: SchemaVersion) -> Result<Catalog, ReleaseCatalogError> {
    let edition = release_edition(version).ok_or(ReleaseCatalogError::NoCatalog(version))?;
    let profile = match edition {
        CatalogEdition::Ifc2x3Tc1 => CatalogProfile::Official,
        _ => CatalogProfile::Corrected,
    };
    #[cfg(feature = "property-catalog")]
    {
        ifc_template_catalog::embedded::load_catalog(edition, profile).map_err(|error| {
            ReleaseCatalogError::Unreadable {
                edition,
                reason: error.to_string(),
            }
        })
    }
    #[cfg(not(feature = "property-catalog"))]
    {
        use ifc_template_catalog::runtime::{load_catalog, RuntimeCatalogError};
        load_catalog(edition, profile).map_err(|error| match error {
            RuntimeCatalogError::NotInstalled(edition) => {
                ReleaseCatalogError::NotInstalled { edition }
            }
            other => ReleaseCatalogError::Unreadable {
                edition,
                reason: other.to_string(),
            },
        })
    }
}

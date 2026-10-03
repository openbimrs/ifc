//! The PSD/QTO catalog that property edits check `Pset_` and `Qto_` sets
//! against (ADR 0017; #123, #318).
//!
//! A build gets the catalog one of two ways:
//!
//! - `property-catalog` (default; the C and Python bindings) embeds every
//!   edition. Nothing needs loading: [`is_loaded`] is `true` and [`load`]
//!   accepts the bytes without reading them.
//! - `property-catalog-runtime` (the npm package) embeds none. The host
//!   reads an edition's snapshot file, named by [`file_name`], and hands its
//!   bytes to [`load`], which checks them against the edition's pinned
//!   SHA-256 and keeps the catalog for the process (for WebAssembly, the
//!   module instance). Until then a write to a `Pset_`/`Qto_` set of that
//!   release is refused with `catalog-not-loaded`; removals and other sets
//!   are not affected.
//!
//! Without either feature every function here refuses with
//! `feature-disabled`, as such a write does.
//!
//! A release is named as a file header names it: `IFC2X3`, `IFC4` or
//! `IFC4X3` (`IFC4X3_ADD2`), any case. Each reads one edition: IFC2X3 TC1,
//! IFC4 ADD2 TC1 or IFC4X3 ADD2. Any other release has no catalog and is
//! refused with `unsupported-schema`.

use crate::BindingError;

/// The releases that have a catalog edition, in edition order.
pub const RELEASES: [&str; 3] = ["IFC2X3", "IFC4", "IFC4X3"];

#[cfg(any(feature = "property-catalog", feature = "property-catalog-runtime"))]
fn edition(
    release: &str,
) -> Result<ifc::property_catalog::definition::CatalogEdition, BindingError> {
    use ifc::property_catalog::definition::CatalogEdition;
    match ifc::SchemaVersion::from_header_token(release) {
        Some(ifc::SchemaVersion::Ifc2x3) => Ok(CatalogEdition::Ifc2x3Tc1),
        Some(ifc::SchemaVersion::Ifc4) => Ok(CatalogEdition::Ifc4Add2Tc1),
        Some(ifc::SchemaVersion::Ifc4x3) => Ok(CatalogEdition::Ifc4x3Add2),
        _ => Err(BindingError::UnsupportedSchema(format!(
            "{release}: no PSD/QTO catalog edition (IFC2X3, IFC4 or IFC4X3)"
        ))),
    }
}

/// The file name of `release`'s catalog snapshot, such as
/// `ifc4x3-add2.bin`; the npm package ships it as `catalog/<name>`.
pub fn file_name(release: &str) -> Result<&'static str, BindingError> {
    #[cfg(any(feature = "property-catalog", feature = "property-catalog-runtime"))]
    {
        Ok(ifc::property_catalog::snapshot::file_name(edition(
            release,
        )?))
    }
    #[cfg(not(any(feature = "property-catalog", feature = "property-catalog-runtime")))]
    {
        let _ = release;
        Err(BindingError::FeatureDisabled("property-catalog"))
    }
}

/// Whether a write to `release`'s catalog sets can be checked now.
pub fn is_loaded(release: &str) -> Result<bool, BindingError> {
    #[cfg(feature = "property-catalog")]
    {
        edition(release).map(|_| true)
    }
    #[cfg(all(
        feature = "property-catalog-runtime",
        not(feature = "property-catalog")
    ))]
    {
        Ok(ifc::property_catalog::runtime::is_installed(edition(
            release,
        )?))
    }
    #[cfg(not(any(feature = "property-catalog", feature = "property-catalog-runtime")))]
    {
        let _ = release;
        Err(BindingError::FeatureDisabled("property-catalog"))
    }
}

/// Load `release`'s catalog from its snapshot bytes: the per-edition file
/// [`file_name`] names, or the container of every edition. Refused with
/// `invalid-value` when the bytes are not the pinned snapshot (another
/// edition's file, another version, a damaged download); nothing is loaded
/// then. Loading an edition again is a no-op.
pub fn load(release: &str, bytes: &[u8]) -> Result<(), BindingError> {
    #[cfg(feature = "property-catalog")]
    {
        // Embedded: every edition is present already.
        let _ = bytes;
        edition(release).map(|_| ())
    }
    #[cfg(all(
        feature = "property-catalog-runtime",
        not(feature = "property-catalog")
    ))]
    {
        let edition = edition(release)?;
        ifc::property_catalog::runtime::install(edition, bytes)
            .map_err(|error| BindingError::InvalidValue(format!("{release} catalog: {error}")))
    }
    #[cfg(not(any(feature = "property-catalog", feature = "property-catalog-runtime")))]
    {
        let _ = (release, bytes);
        Err(BindingError::FeatureDisabled("property-catalog"))
    }
}

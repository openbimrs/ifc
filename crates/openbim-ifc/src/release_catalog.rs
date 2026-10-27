//! The PSD/QTO catalog a release reads: the one policy behind property
//! edits ([`apply_property_edits`](crate::apply_property_edits)) and
//! validation ([`validate`](crate::validate)), public so an authoring check
//! made before a product exists reads the same catalog (#566).
//!
//! IFC2X3 reads its TC1 catalog as published; IFC4 reads IFC4 ADD2 TC1 and
//! IFC4X3 reads IFC4X3 ADD2, both with the built-in corrections. IFC4X1 and
//! IFC4X2 have no catalog. With `property-catalog` the catalog is the
//! embedded one; with only `property-catalog-runtime` it is the one the host
//! installed.

#[cfg(all(
    feature = "schema-api",
    any(feature = "property-catalog", feature = "property-catalog-runtime")
))]
mod policy;
#[cfg(all(
    feature = "schema-api",
    any(feature = "property-catalog", feature = "property-catalog-runtime")
))]
pub use policy::{release_catalog, release_edition, ReleaseCatalogError};

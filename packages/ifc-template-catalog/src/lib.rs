//! Versioned IFC PSD/QTO template catalogs.
//!
//! This crate owns external standard-library metadata. Authored IFC property
//! and quantity instances remain in `ifc-properties`.

mod archive;

pub mod catalog;
pub mod compliance;
pub mod definition;
pub mod diagnostic;
#[cfg(feature = "embedded")]
pub mod embedded;
pub mod overlay;
pub mod query;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(feature = "generation")]
#[doc(hidden)]
pub mod generation {
    pub use crate::archive::{decode_catalog, encode_catalog, ArchiveError};
}

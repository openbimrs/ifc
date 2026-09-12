//! Versioned IFC PSD/QTO template catalogs.
//!
//! This crate owns external standard-library metadata. Authored IFC property
//! and quantity instances remain in `ifc-properties`.

// Documentation debt: this crate does not yet document every public item,
// so it cannot join [workspace.lints] (missing_docs = deny) yet. The
// remaining count is budgeted in scripts/check-missing-docs.py, which fails
// if it grows. Delete this attribute once the count reaches zero and add
// `[lints] workspace = true` to Cargo.toml instead.
#![allow(missing_docs)]

mod archive;

pub mod catalog;
pub mod compliance;
pub mod definition;
pub mod diagnostic;
#[cfg(feature = "embedded")]
pub mod embedded;
pub mod export;
pub mod overlay;
pub mod query;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(feature = "generation")]
#[doc(hidden)]
pub mod generation {
    pub use crate::archive::{decode_catalog, encode_catalog, ArchiveError};
}

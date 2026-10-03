//! Versioned IFC PSD/QTO template catalogs.
//!
//! This crate owns external standard-library metadata. Authored IFC property
//! and quantity instances remain in `ifc-properties`.

mod pack;

pub mod catalog;
pub mod compliance;
pub mod definition;
pub mod diagnostic;
#[cfg(feature = "embedded")]
pub mod embedded;
pub mod export;
pub mod overlay;
pub mod query;
#[cfg(feature = "runtime")]
pub mod runtime;
pub mod snapshot;

#[cfg(feature = "xml")]
pub mod xml;

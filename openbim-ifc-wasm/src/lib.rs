//! WebAssembly bindings for the IFC facade (ADR 0013, #34).
//!
//! Reads, edits and writes IFC STEP files from JavaScript. The surface is the
//! record model: an [`IfcModel`] holds entities, each a type name plus
//! positional attributes. Attribute values cross the boundary in a lossless
//! tagged encoding, described in [`value`], so that a file read and written
//! back through JavaScript is unchanged.
//!
//! This crate adds calling-convention glue only. IFC behaviour belongs in
//! `openbim-ifc`; if the bindings need more, the facade grows first.

mod error;
mod model;
pub mod value;

pub use error::BindingError;
pub use model::IfcModel;

//! WebAssembly bindings for the IFC facade (ADR 0013, #34).
//!
//! Reads, edits and writes IFC STEP files from JavaScript. The surface is the
//! record model: an `IfcModel` holds entities, each a type name plus
//! positional attributes. Attribute values cross the boundary in the lossless
//! tagged encoding of `openbim_ifc_binding_core::value`, so a file read and
//! written back through JavaScript is unchanged.
//!
//! Every operation lives in `openbim-ifc-binding-core`, shared with the C and
//! Python bindings. This crate converts JS arguments and results only, and is
//! empty outside `wasm32`.

#[cfg(target_arch = "wasm32")]
mod error;
#[cfg(target_arch = "wasm32")]
mod model;
#[cfg(target_arch = "wasm32")]
mod value;

#[cfg(target_arch = "wasm32")]
pub use model::IfcModel;

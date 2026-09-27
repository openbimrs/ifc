//! Host-independent core of the IFC language bindings (ADR 0013, #34).
//!
//! The WebAssembly, C and Python bindings each wrap [`IfcModel`] and move
//! [`value::Tagged`] values into and out of their host. The operations, the
//! lossless value encoding and the error codes live here once, so the three
//! hosts cannot drift apart: a fix or a new operation lands in one place and
//! every binding exposes it.
//!
//! This crate adds no IFC behaviour of its own. It is a thin, host-shaped
//! view of `openbim-ifc`; if a binding needs more, the facade grows first.

mod error;
mod model;
pub mod value;

pub use error::BindingError;
pub use model::IfcModel;

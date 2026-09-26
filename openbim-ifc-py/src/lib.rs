//! Python bindings for the IFC facade (ADR 0013, #39).
//!
//! The compiled module is `openbim_ifc._native`; the public API is the pure
//! Python package `openbim_ifc`, which wraps it and defines the value
//! classes. Values cross as plain dicts in the tagged encoding shared with
//! the WASM and C bindings; `openbim_ifc` turns them into frozen dataclasses.
//!
//! This crate adds calling-convention glue only. IFC behaviour belongs in
//! `openbim-ifc`, shared binding behaviour in `openbim-ifc-binding-core`.

mod convert;
mod error;
mod model;

use pyo3::prelude::*;

/// Opt-in allocator (feature `rusty_alloc`, off by default). It only covers
/// this extension's Rust allocations; Python's own allocator is untouched.
/// See #49 for the measurements.
#[cfg(feature = "rusty_alloc")]
#[global_allocator]
static ALLOCATOR: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;

/// The `openbim_ifc._native` extension module.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<model::NativeModel>()?;
    module.add("IfcError", module.py().get_type::<error::IfcError>())?;
    Ok(())
}

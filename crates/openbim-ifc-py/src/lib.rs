//! Python bindings for the IFC facade (ADR 0013, #39).
//!
//! The compiled module is `openbim_ifc._native`; the public API is the pure
//! Python package `openbim_ifc`, which wraps it and defines the value
//! classes. Values cross as plain dicts in the tagged encoding shared with
//! the WASM and C bindings; `openbim_ifc` turns them into frozen dataclasses.
//!
//! This crate adds calling-convention glue only. IFC behaviour belongs in
//! `openbim-ifc`, shared binding behaviour in `openbim-ifc-binding-core`.

mod authoring;
mod convert;
mod edits;
mod error;
mod model;
mod records;

use pyo3::prelude::*;

/// Opt-in allocator (feature `rusty_alloc`, off by default). It only covers
/// this extension's Rust allocations; Python's own allocator is untouched.
/// See #49 for the measurements.
#[cfg(feature = "rusty_alloc")]
#[global_allocator]
static ALLOCATOR: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;

/// A new random `GlobalId` (RFC 9562 version 4), #463.
#[pyfunction]
fn new_guid() -> PyResult<String> {
    openbim_ifc_binding_core::new_guid().map_err(error::py_err)
}

/// The reproducible `GlobalId` named by `parts` (RFC 9562 version 8).
#[pyfunction]
fn name_based_guid(parts: Vec<String>) -> String {
    openbim_ifc_binding_core::name_based_guid(&parts)
}

/// A selector in its canonical form (#464); raises `invalid-selector`.
#[pyfunction]
fn canonical_selector(query: &str) -> PyResult<String> {
    openbim_ifc_binding_core::canonical_selector(query).map_err(error::py_err)
}

/// Convert `value` between two units named as text (#469).
#[pyfunction]
fn convert_unit(value: f64, from: &str, to: &str) -> PyResult<f64> {
    openbim_ifc_binding_core::convert_unit(value, from, to).map_err(error::py_err)
}

/// A length in `unit` as text (#469).
#[pyfunction]
#[pyo3(signature = (value, unit, style, places, display_unit = None, suppress_zero_inches = false))]
fn format_length(
    value: f64,
    unit: &str,
    style: &str,
    places: u32,
    display_unit: Option<&str>,
    suppress_zero_inches: bool,
) -> PyResult<String> {
    openbim_ifc_binding_core::format_length(
        value,
        unit,
        style,
        places,
        display_unit,
        suppress_zero_inches,
    )
    .map_err(error::py_err)
}

/// The `openbim_ifc._native` extension module.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<model::NativeModel>()?;
    module.add_function(wrap_pyfunction!(new_guid, module)?)?;
    module.add_function(wrap_pyfunction!(name_based_guid, module)?)?;
    module.add_function(wrap_pyfunction!(canonical_selector, module)?)?;
    module.add_function(wrap_pyfunction!(convert_unit, module)?)?;
    module.add_function(wrap_pyfunction!(format_length, module)?)?;
    module.add(
        "SELECTOR_GRAMMAR_VERSION",
        openbim_ifc_binding_core::SELECTOR_GRAMMAR_VERSION,
    )?;
    module.add("IfcError", module.py().get_type::<error::IfcError>())?;
    Ok(())
}

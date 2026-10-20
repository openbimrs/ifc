//! Unit conversion and length formatting without a model (#469).

use pyo3::prelude::*;

use crate::error;

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

/// Add the unit functions to the extension module.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(convert_unit, module)?)?;
    module.add_function(wrap_pyfunction!(format_length, module)?)
}

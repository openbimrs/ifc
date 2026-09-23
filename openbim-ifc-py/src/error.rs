//! `IfcError`: one exception type carrying the shared stable `code`.

use openbim_ifc_binding_core::BindingError;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(
    _native,
    IfcError,
    PyException,
    "A failed IFC operation. `code` is stable across bindings: parse, write, missing-entity, invalid-value, out-of-range, unsupported-schema."
);

/// Raise `error` as an `IfcError` with `.code` set.
pub fn py_err(error: BindingError) -> PyErr {
    let err = IfcError::new_err(error.to_string());
    Python::attach(|py| {
        // Setting an attribute on a fresh exception instance cannot fail in
        // practice; if it did, the message alone is still correct.
        let _ = err.value(py).setattr("code", error.code());
    });
    err
}

//! The model methods that build binding-core options and answer with a
//! record, kept out of `model.rs` so that file stays a thin method list.

use openbim_ifc_binding_core::{
    IfcModel, LengthUnitOptions, PurgeOptions, RemoveProductOptions, ToRecord,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::error::py_err;
use crate::records;

/// `remove_product` with the native keywords.
pub fn remove_product<'py>(
    py: Python<'py>,
    model: &mut IfcModel,
    id: u64,
    flags: (bool, bool, bool),
) -> PyResult<Bound<'py, PyDict>> {
    let mut options = RemoveProductOptions::new();
    (
        options.rebase_placements,
        options.untype_occurrences,
        options.dry_run,
    ) = flags;
    let report = model.remove_product(id, options).map_err(py_err)?;
    records::record_to_py(py, &report.to_record())
}

/// `purge` with the native keywords.
pub fn purge<'py>(
    py: Python<'py>,
    model: &mut IfcModel,
    classes: Vec<String>,
    flags: (bool, bool, bool),
) -> PyResult<Bound<'py, PyDict>> {
    let mut options = PurgeOptions::new();
    options.classes = classes;
    (
        options.include_roots,
        options.include_owner_history,
        options.dry_run,
    ) = flags;
    let report = model.purge(options).map_err(py_err)?;
    records::record_to_py(py, &report.to_record())
}

/// `remove_representation`.
pub fn remove_representation<'py>(
    py: Python<'py>,
    model: &mut IfcModel,
    id: u64,
    dry_run: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let report = model.remove_representation(id, dry_run).map_err(py_err)?;
    records::record_to_py(py, &report.to_record())
}

/// `convert_length_unit` with the native keywords.
pub fn convert_length_unit<'py>(
    py: Python<'py>,
    model: &mut IfcModel,
    target: &str,
    flags: (bool, bool, bool),
) -> PyResult<Bound<'py, PyDict>> {
    let mut options = LengthUnitOptions::new();
    (
        options.area_and_volume,
        options.remove_replaced_units,
        options.dry_run,
    ) = flags;
    let report = model.convert_length_unit(target, options).map_err(py_err)?;
    records::record_to_py(py, &report.to_record())
}

/// `history`.
pub fn history<'py>(py: Python<'py>, model: &IfcModel) -> PyResult<Bound<'py, PyDict>> {
    records::record_to_py(py, &model.history().to_record())
}

/// `undo` or `redo`, by `forward`.
pub fn step<'py>(
    py: Python<'py>,
    model: &mut IfcModel,
    forward: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let change = if forward { model.redo() } else { model.undo() }.map_err(py_err)?;
    records::record_to_py(py, &change.to_record())
}

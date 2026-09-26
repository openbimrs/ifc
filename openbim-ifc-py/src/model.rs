//! The native model class, wrapped by `openbim_ifc.IfcModel` in Python.

use openbim_ifc_binding_core::IfcModel;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use crate::convert::{from_py, to_py};
use crate::error::py_err;

/// Native half of `openbim_ifc.IfcModel`; not part of the public API.
///
/// Usable from any thread: the model is `Send + Sync`, the GIL serialises
/// calls, and pyo3's runtime borrow check turns a conflicting borrow into a
/// Python `RuntimeError`. Not `unsendable`, which would make cross-thread
/// use a Rust panic (`PanicException`, uncatchable by `except Exception`).
#[pyclass(module = "openbim_ifc._native", name = "NativeModel")]
pub struct NativeModel {
    inner: IfcModel,
}

#[pymethods]
impl NativeModel {
    /// An empty model.
    #[new]
    fn new() -> Self {
        Self {
            inner: IfcModel::empty(),
        }
    }

    /// Parse STEP bytes. Parsing releases the GIL, so other Python threads
    /// keep running while a large file loads. The one copy made to release
    /// the GIL becomes the model's source; nothing is copied again.
    #[staticmethod]
    fn parse(py: Python<'_>, data: &[u8]) -> PyResult<Self> {
        let owned = data.to_vec();
        let inner = py
            .detach(move || IfcModel::parse_owned(owned))
            .map_err(py_err)?;
        Ok(Self { inner })
    }

    /// Read a STEP file from disk, releasing the GIL. `mapped` reads it
    /// through a memory mapping; see `openbim_ifc.IfcModel.open`.
    #[staticmethod]
    #[pyo3(signature = (path, mapped = false))]
    fn open(py: Python<'_>, path: std::path::PathBuf, mapped: bool) -> PyResult<Self> {
        let inner = py
            .detach(move || {
                if mapped {
                    // SAFETY: the Python API documents the contract -- the
                    // file must stay unchanged while the model is alive --
                    // and only reaches here when the caller asked for it.
                    unsafe { IfcModel::open_mapped(&path) }
                } else {
                    IfcModel::open(&path)
                }
            })
            .map_err(py_err)?;
        Ok(Self { inner })
    }

    /// Serialize as STEP bytes.
    fn write<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.write().map_err(py_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[getter]
    fn schema(&self) -> Option<String> {
        self.inner.schema().map(str::to_owned)
    }

    fn diagnostics(&self) -> Vec<String> {
        self.inner.diagnostics()
    }

    fn ids(&self) -> Vec<u64> {
        self.inner.ids()
    }

    fn ids_of_type(&self, type_name: &str) -> Vec<u64> {
        self.inner.ids_of_type(type_name)
    }

    fn ids_of_type_including_subtypes(&self, type_name: &str) -> PyResult<Vec<u64>> {
        self.inner
            .ids_of_type_including_subtypes(type_name)
            .map_err(py_err)
    }

    fn type_of(&self, id: u64) -> PyResult<String> {
        self.inner.type_of(id).map(str::to_owned).map_err(py_err)
    }

    fn attributes<'py>(&self, py: Python<'py>, id: u64) -> PyResult<Bound<'py, PyList>> {
        let values = self.inner.attributes(id).map_err(py_err)?;
        let list = PyList::empty(py);
        for value in &values {
            list.append(to_py(py, value)?)?;
        }
        Ok(list)
    }

    fn attribute<'py>(
        &self,
        py: Python<'py>,
        id: u64,
        index: usize,
    ) -> PyResult<Bound<'py, PyDict>> {
        to_py(py, &self.inner.attribute(id, index).map_err(py_err)?)
    }

    fn set_attribute<'py>(
        &mut self,
        py: Python<'py>,
        id: u64,
        index: usize,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let value = from_py(value).map_err(py_err)?;
        to_py(
            py,
            &self.inner.set_attribute(id, index, value).map_err(py_err)?,
        )
    }

    fn add(&mut self, type_name: &str, attributes: &Bound<'_, PyAny>) -> PyResult<u64> {
        let values = attributes
            .try_iter()
            .map_err(|_| {
                py_err(openbim_ifc_binding_core::BindingError::InvalidValue(
                    "attributes must be iterable".into(),
                ))
            })?
            .map(|item| from_py(&item?).map_err(py_err))
            .collect::<PyResult<Vec<_>>>()?;
        self.inner.add(type_name, values).map_err(py_err)
    }

    fn remove(&mut self, id: u64) -> PyResult<()> {
        self.inner.remove(id).map_err(py_err)
    }

    fn dangling_references(&self) -> Vec<(u64, u64)> {
        self.inner.dangling_references()
    }
}

//! The native model class, wrapped by `openbim_ifc.IfcModel` in Python.

use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::{IfcModel, ParseOptions, ToRecord};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyList};

use crate::convert::{from_py, to_py};
use crate::edits;
use crate::error::py_err;
use crate::records;

/// Parse options from the native keywords; `None` flags are `False`.
fn options(
    py: Python<'_>,
    on_malformed: &str,
    check_references: Option<&Bound<'_, PyAny>>,
    accept_real_without_point: Option<&Bound<'_, PyAny>>,
) -> PyResult<ParseOptions> {
    let off = PyBool::new(py, false).to_owned().into_any();
    records::parse_options(
        on_malformed,
        check_references.unwrap_or(&off),
        accept_real_without_point.unwrap_or(&off),
    )
    .map_err(py_err)
}

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
    ///
    /// The keywords are the read options (`openbim_ifc.ParseOptions`);
    /// their defaults are the strict read.
    #[staticmethod]
    #[pyo3(signature = (data, on_malformed = "abort", check_references = None, accept_real_without_point = None))]
    fn parse(
        py: Python<'_>,
        data: &[u8],
        on_malformed: &str,
        check_references: Option<&Bound<'_, PyAny>>,
        accept_real_without_point: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let options = options(
            py,
            on_malformed,
            check_references,
            accept_real_without_point,
        )?;
        let owned = data.to_vec();
        let inner = py
            .detach(move || IfcModel::parse_owned_with(owned, options))
            .map_err(py_err)?;
        Ok(Self { inner })
    }

    /// Parse an ifcXML document, releasing the GIL: the native layout
    /// without `xsd_profile`, else that release's XSD layout.
    #[staticmethod]
    #[pyo3(signature = (data, xsd_profile = None))]
    fn parse_ifcxml(py: Python<'_>, data: &[u8], xsd_profile: Option<String>) -> PyResult<Self> {
        let owned = data.to_vec();
        let inner = py
            .detach(move || IfcModel::parse_ifcxml(&owned, xsd_profile.as_deref()))
            .map_err(py_err)?;
        Ok(Self { inner })
    }

    /// Read a STEP file from disk, releasing the GIL. `mapped` reads it
    /// through a memory mapping; see `openbim_ifc.IfcModel.open`.
    #[staticmethod]
    #[pyo3(signature = (path, mapped = false, on_malformed = "abort", check_references = None, accept_real_without_point = None))]
    fn open(
        py: Python<'_>,
        path: std::path::PathBuf,
        mapped: bool,
        on_malformed: &str,
        check_references: Option<&Bound<'_, PyAny>>,
        accept_real_without_point: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let options = options(
            py,
            on_malformed,
            check_references,
            accept_real_without_point,
        )?;
        let inner = py
            .detach(move || {
                if mapped {
                    // SAFETY: the Python API documents the contract -- the
                    // file must stay unchanged while the model is alive --
                    // and only reaches here when the caller asked for it.
                    unsafe { IfcModel::open_mapped_with(&path, options) }
                } else {
                    IfcModel::open_with(&path, options)
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

    /// Serialize as ifcXML bytes.
    #[pyo3(signature = (xsd_profile = None))]
    fn write_ifcxml<'py>(
        &self,
        py: Python<'py>,
        xsd_profile: Option<&str>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.write_ifcxml(xsd_profile).map_err(py_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// The STEP header as a dict.
    fn header<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        records::header_to_py(py, &self.inner.header())
    }

    /// Replace the STEP header from a dict with every field.
    fn set_header(&mut self, header: &Bound<'_, PyAny>) -> PyResult<()> {
        let header = records::header_from_py(header).map_err(py_err)?;
        self.inner.set_header(header);
        Ok(())
    }

    /// Validate against the declared schema, releasing the GIL.
    #[pyo3(signature = (max_findings = None))]
    fn validate<'py>(
        &self,
        py: Python<'py>,
        max_findings: Option<usize>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let inner = &self.inner;
        let report = py.detach(|| inner.validate(max_findings)).map_err(py_err)?;
        records::report_to_py(py, &report)
    }

    /// Products no viewer will draw, as dicts.
    fn unreachable_products<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let products = self.inner.unreachable_products().map_err(py_err)?;
        records::unreachable_to_py(py, &products)
    }

    /// Property sets of `id`, its own then its type's, as record dicts.
    fn property_sets<'py>(&self, py: Python<'py>, id: u64) -> PyResult<Bound<'py, PyList>> {
        let sets = self.inner.property_sets(id).map_err(py_err)?;
        records::records_to_py(py, &to_records(&sets))
    }

    /// The effective unit of a `measure_type` value, as a record dict.
    #[pyo3(signature = (measure_type, unit = None))]
    fn resolve_unit<'py>(
        &self,
        py: Python<'py>,
        measure_type: &str,
        unit: Option<u64>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let unit = self
            .inner
            .resolve_unit(measure_type, unit)
            .map_err(py_err)?;
        records::record_to_py(py, &unit.to_record())
    }

    /// Apply property edits (dicts, see `edits`) as one checked
    /// transaction; the `PropertyEditResult` record dict.
    fn set_properties<'py>(
        &mut self,
        py: Python<'py>,
        edits: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let edits = edits::edits_from_py(edits).map_err(py_err)?;
        let result = self.inner.set_properties(edits).map_err(py_err)?;
        records::record_to_py(py, &result.to_record())
    }

    /// Write one value; the id of the entity holding it.
    #[pyo3(signature = (object, set, name, value, set_type = None))]
    fn set_property(
        &mut self,
        object: u64,
        set: &str,
        name: &str,
        value: &Bound<'_, PyAny>,
        set_type: Option<String>,
    ) -> PyResult<u64> {
        let value = from_py(value).map_err(py_err)?;
        self.inner
            .set_property(object, set, name, value, set_type)
            .map_err(py_err)
    }

    /// Remove one property from the object's own set.
    fn remove_property(&mut self, object: u64, set: &str, name: &str) -> PyResult<()> {
        self.inner
            .remove_property(object, set, name)
            .map_err(py_err)
    }

    /// The spatial containment tree, as a record dict.
    fn spatial_tree<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let tree = self.inner.spatial_tree().map_err(py_err)?;
        records::record_to_py(py, &tree.to_record())
    }

    /// Classifications of `id`, its own then its type's, as record dicts.
    fn classifications<'py>(&self, py: Python<'py>, id: u64) -> PyResult<Bound<'py, PyList>> {
        let classes = self.inner.classifications(id).map_err(py_err)?;
        records::records_to_py(py, &to_records(&classes))
    }

    /// The material association of `id`, as a record dict, or `None`.
    fn material<'py>(&self, py: Python<'py>, id: u64) -> PyResult<Option<Bound<'py, PyDict>>> {
        let material = self.inner.material(id).map_err(py_err)?;
        material
            .map(|material| records::record_to_py(py, &material.to_record()))
            .transpose()
    }

    /// Every system and the memberships not honoured, as a record dict.
    fn systems<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let systems = self.inner.systems().map_err(py_err)?;
        records::record_to_py(py, &systems.to_record())
    }

    /// Every cost schedule and item, as a record dict.
    fn cost<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let cost = self.inner.cost().map_err(py_err)?;
        records::record_to_py(py, &cost.to_record())
    }

    /// Every coordinate operation, resolved, as record dicts.
    fn georeferencing<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let maps = self.inner.georeferencing().map_err(py_err)?;
        records::records_to_py(py, &to_records(&maps))
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

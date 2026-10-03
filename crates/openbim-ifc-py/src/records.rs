//! Moves the #244 records -- parse options, the header, validation reports
//! and unreachable products -- into and out of Python dicts. The
//! pure-Python layer turns them into frozen dataclasses
//! (`openbim_ifc.records`). Every malformed input is an `invalid-value`
//! [`BindingError`], as in the other hosts.

use openbim_ifc_binding_core::header::Header;
use openbim_ifc_binding_core::{
    BindingError, OnMalformed, ParseOptions, UnreachableProduct, ValidationReport,
};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyString, PyTuple};

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// Options from the native keywords; `on_malformed` is `abort` or `skip`.
pub fn parse_options(
    on_malformed: &str,
    check_references: &Bound<'_, PyAny>,
    accept_real_without_point: &Bound<'_, PyAny>,
) -> Result<ParseOptions, BindingError> {
    let flag = |value: &Bound<'_, PyAny>, name: &str| {
        value
            .cast::<PyBool>()
            .map(|b| b.is_true())
            .map_err(|_| invalid(format!("`{name}` must be a bool")))
    };
    Ok(ParseOptions {
        on_malformed: OnMalformed::parse(on_malformed)?,
        check_references: flag(check_references, "check_references")?,
        accept_real_without_point: flag(accept_real_without_point, "accept_real_without_point")?,
    })
}

/// The header as a dict of the dataclass's field names.
pub fn header_to_py<'py>(py: Python<'py>, header: &Header) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("description", &header.description)?;
    dict.set_item("implementation_level", &header.implementation_level)?;
    dict.set_item("name", &header.name)?;
    dict.set_item("time_stamp", &header.time_stamp)?;
    dict.set_item("author", &header.author)?;
    dict.set_item("organization", &header.organization)?;
    dict.set_item("preprocessor_version", &header.preprocessor_version)?;
    dict.set_item("originating_system", &header.originating_system)?;
    dict.set_item("authorization", &header.authorization)?;
    dict.set_item("schema", &header.schema)?;
    Ok(dict)
}

/// A header dict; every key is required.
pub fn header_from_py(value: &Bound<'_, PyAny>) -> Result<Header, BindingError> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| invalid("a header must be a dict"))?;
    let field = |key: &str| -> Result<Bound<'_, PyAny>, BindingError> {
        dict.get_item(key)
            .map_err(|_| invalid(format!("cannot read header `{key}`")))?
            .ok_or_else(|| invalid(format!("header `{key}` is missing")))
    };
    let text = |key: &str| -> Result<String, BindingError> {
        Ok(field(key)?
            .cast::<PyString>()
            .map_err(|_| invalid(format!("header `{key}` must be a str")))?
            .to_string())
    };
    let texts = |key: &str| -> Result<Vec<String>, BindingError> {
        let value = field(key)?;
        let items: Vec<Bound<'_, PyAny>> = if let Ok(list) = value.cast::<PyList>() {
            list.iter().collect()
        } else if let Ok(tuple) = value.cast::<PyTuple>() {
            tuple.iter().collect()
        } else {
            return Err(invalid(format!(
                "header `{key}` must be a list or tuple of str"
            )));
        };
        items
            .iter()
            .map(|item| {
                item.cast::<PyString>()
                    .map(ToString::to_string)
                    .map_err(|_| invalid(format!("header `{key}` must hold str only")))
            })
            .collect()
    };
    Ok(Header {
        description: texts("description")?,
        implementation_level: text("implementation_level")?,
        name: text("name")?,
        time_stamp: text("time_stamp")?,
        author: texts("author")?,
        organization: texts("organization")?,
        preprocessor_version: text("preprocessor_version")?,
        originating_system: text("originating_system")?,
        authorization: text("authorization")?,
        schema: texts("schema")?,
    })
}

/// A validation report as a dict, findings as a list of dicts.
pub fn report_to_py<'py>(
    py: Python<'py>,
    report: &ValidationReport,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("conformant", report.conformant)?;
    dict.set_item("truncated", report.truncated)?;
    dict.set_item("errors", report.summary.errors)?;
    dict.set_item("evaluation_errors", report.summary.evaluation_errors)?;
    dict.set_item("warnings", report.summary.warnings)?;
    dict.set_item("unsupported", report.summary.unsupported)?;
    let findings = PyList::empty(py);
    for finding in &report.findings {
        let row = PyDict::new(py);
        row.set_item("severity", &finding.severity)?;
        row.set_item("rule", &finding.rule)?;
        row.set_item("entity", finding.entity)?;
        row.set_item("attribute_index", finding.attribute_index)?;
        row.set_item("attribute_name", &finding.attribute_name)?;
        row.set_item("path", &finding.path)?;
        row.set_item("message", &finding.message)?;
        findings.append(row)?;
    }
    dict.set_item("findings", findings)?;
    Ok(dict)
}

/// Unreachable products as a list of dicts.
pub fn unreachable_to_py<'py>(
    py: Python<'py>,
    products: &[UnreachableProduct],
) -> PyResult<Bound<'py, PyList>> {
    let list = PyList::empty(py);
    for product in products {
        let row = PyDict::new(py);
        row.set_item("id", product.id)?;
        row.set_item("reason", &product.reason)?;
        row.set_item("found_views", &product.found_views)?;
        row.set_item("message", &product.message)?;
        list.append(row)?;
    }
    Ok(list)
}

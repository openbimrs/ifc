//! Moves [`Tagged`] values into and out of Python dicts.
//!
//! The dict shape is the shared tagged encoding: `{"kind": "ref", "id": 5}`.
//! The pure-Python layer turns these into frozen dataclasses. Every malformed
//! input is an `invalid-value` [`BindingError`], never a panic or a guess.

use openbim_ifc_binding_core::value::{Kind, Tagged, MAX_NESTING};
use openbim_ifc_binding_core::BindingError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// Encode a value as a dict.
pub fn to_py<'py>(py: Python<'py>, value: &Tagged) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("kind", value.kind().as_str())?;
    match value {
        Tagged::Null | Tagged::Derived | Tagged::Unknown => {}
        Tagged::Bool(b) => dict.set_item("value", *b)?,
        Tagged::Integer(i) => dict.set_item("value", *i)?,
        Tagged::Real(r) => dict.set_item("value", *r)?,
        Tagged::Text(s) | Tagged::Binary(s) | Tagged::Enum(s) => dict.set_item("value", s)?,
        Tagged::Ref(id) => dict.set_item("id", *id)?,
        Tagged::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(to_py(py, item)?)?;
            }
            dict.set_item("items", list)?;
        }
        Tagged::Typed { type_name, value } => {
            dict.set_item("type", type_name)?;
            dict.set_item("value", to_py(py, value)?)?;
        }
    }
    Ok(dict)
}

/// Decode a dict, rejecting anything outside the encoding.
pub fn from_py(value: &Bound<'_, PyAny>) -> Result<Tagged, BindingError> {
    from_py_at(value, 0)
}

fn from_py_at(value: &Bound<'_, PyAny>, depth: usize) -> Result<Tagged, BindingError> {
    if depth > MAX_NESTING {
        return Err(invalid(format!("nesting deeper than {MAX_NESTING}")));
    }
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| invalid("expected a dict with a `kind` key"))?;
    let kind: String = field(dict, "kind")?
        .cast::<PyString>()
        .map_err(|_| invalid("`kind` must be a str"))?
        .to_string();
    Ok(match Kind::parse(&kind)? {
        Kind::Null => Tagged::Null,
        Kind::Derived => Tagged::Derived,
        Kind::Unknown => Tagged::Unknown,
        Kind::Bool => Tagged::Bool(boolean(&field(dict, "value")?)?),
        Kind::Integer => Tagged::Integer(integer(&field(dict, "value")?, "integer `value`")?),
        Kind::Real => Tagged::Real(real(&field(dict, "value")?)?),
        Kind::Text => Tagged::Text(string(dict, "value")?),
        Kind::Binary => Tagged::Binary(string(dict, "value")?),
        Kind::Enum => Tagged::Enum(string(dict, "value")?),
        Kind::Ref => {
            let id = integer(&field(dict, "id")?, "ref `id`")?;
            Tagged::Ref(u64::try_from(id).map_err(|_| invalid("ref `id` must be positive"))?)
        }
        Kind::List => {
            let items = field(dict, "items")?;
            let items = items
                .cast::<PyList>()
                .map(|l| l.iter().collect::<Vec<_>>())
                .or_else(|_| items.cast::<PyTuple>().map(|t| t.iter().collect()))
                .map_err(|_| invalid("list `items` must be a list or tuple"))?;
            Tagged::List(
                items
                    .iter()
                    .map(|item| from_py_at(item, depth + 1))
                    .collect::<Result<_, _>>()?,
            )
        }
        Kind::Typed => Tagged::Typed {
            type_name: string(dict, "type")?,
            value: Box::new(from_py_at(&field(dict, "value")?, depth + 1)?),
        },
    })
}

fn field<'py>(dict: &Bound<'py, PyDict>, key: &str) -> Result<Bound<'py, PyAny>, BindingError> {
    dict.get_item(key)
        .map_err(|_| invalid(format!("cannot read `{key}`")))?
        .ok_or_else(|| invalid(format!("missing `{key}`")))
}

fn string(dict: &Bound<'_, PyDict>, key: &str) -> Result<String, BindingError> {
    Ok(field(dict, key)?
        .cast::<PyString>()
        .map_err(|_| invalid(format!("`{key}` must be a str")))?
        .to_string())
}

/// A real `bool`, not any truthy object: `0` or `"yes"` must not become
/// `.F.`/`.T.` silently.
fn boolean(value: &Bound<'_, PyAny>) -> Result<bool, BindingError> {
    value
        .cast::<PyBool>()
        .map(|b| b.is_true())
        .map_err(|_| invalid("bool `value` must be a bool"))
}

/// A Python `int` that fits in 64 bits. `bool` is an `int` subclass in
/// Python, so it is refused explicitly: `True` is not the integer 1 here.
fn integer(value: &Bound<'_, PyAny>, what: &str) -> Result<i64, BindingError> {
    if value.is_instance_of::<PyBool>() || !value.is_instance_of::<PyInt>() {
        return Err(invalid(format!("{what} must be an int")));
    }
    value
        .extract::<i64>()
        .map_err(|_| BindingError::OutOfRange(format!("{what} does not fit in 64 bits")))
}

/// A finite real from a `float` or an `int` (an int is exactly representable
/// or it overflows; neither is silently lossy beyond f64's own precision).
fn real(value: &Bound<'_, PyAny>) -> Result<f64, BindingError> {
    if value.is_instance_of::<PyBool>()
        || !(value.is_instance_of::<PyFloat>() || value.is_instance_of::<PyInt>())
    {
        return Err(invalid("real `value` must be a float"));
    }
    let number: f64 = value
        .extract()
        .map_err(|_| invalid("real `value` must be a float"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(invalid("real `value` must be finite"))
    }
}

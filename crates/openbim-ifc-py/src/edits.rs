//! Moves property edits (#123) from Python dicts into the core.
//!
//! The pure-Python `openbim_ifc.PropertyEdit` sends one dict per edit:
//! `object` (int), `set` and `name` (str), `remove` (bool), and, for a
//! write, `value` (a tagged-value dict) and `set_type` (str or None). Every
//! malformed edit is an `invalid-value` error naming its position.

use openbim_ifc_binding_core::property_edit::{EditAction, PropertyEdit};
use openbim_ifc_binding_core::BindingError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyString};

use crate::convert::from_py;

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// A sequence of edit dicts.
pub fn edits_from_py(value: &Bound<'_, PyAny>) -> Result<Vec<PropertyEdit>, BindingError> {
    value
        .try_iter()
        .map_err(|_| invalid("property edits must be iterable"))?
        .enumerate()
        .map(|(index, item)| {
            let item = item.map_err(|_| invalid(format!("edit {index}: cannot be read")))?;
            edit_from_py(&item).map_err(|error| match error {
                BindingError::InvalidValue(detail) => invalid(format!("edit {index}: {detail}")),
                other => other,
            })
        })
        .collect()
}

fn edit_from_py(item: &Bound<'_, PyAny>) -> Result<PropertyEdit, BindingError> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| invalid("an edit must be a dict"))?;
    let field = |key: &str| -> Result<Option<Bound<'_, PyAny>>, BindingError> {
        Ok(dict
            .get_item(key)
            .map_err(|_| invalid(format!("cannot read `{key}`")))?
            .filter(|value| !value.is_none()))
    };
    let text = |key: &str| -> Result<String, BindingError> {
        field(key)?
            .and_then(|value| value.cast::<PyString>().ok().map(ToString::to_string))
            .ok_or_else(|| invalid(format!("`{key}` must be a str")))
    };
    let object = field("object")?
        .filter(|value| value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>())
        .ok_or_else(|| invalid("`object` must be an int"))?
        .extract::<u64>()
        .map_err(|_| {
            BindingError::OutOfRange("`object` must be a non-negative 64-bit id".into())
        })?;
    let (set, name) = (text("set")?, text("name")?);
    let remove = match field("remove")? {
        None => false,
        Some(value) => value
            .cast::<PyBool>()
            .map(|flag| flag.is_true())
            .map_err(|_| invalid("`remove` must be a bool"))?,
    };
    let value = field("value")?;
    let set_type = field("set_type")?;
    let action = if remove {
        if value.is_some() || set_type.is_some() {
            return Err(invalid("a removal takes no `value` or `set_type`"));
        }
        EditAction::Remove
    } else {
        let value = value.ok_or_else(|| invalid("a write needs a `value`"))?;
        let set_type = set_type
            .map(|value| {
                value
                    .cast::<PyString>()
                    .map(ToString::to_string)
                    .map_err(|_| invalid("`set_type` must be a str"))
            })
            .transpose()?;
        EditAction::Set {
            value: from_py(&value)?,
            set_type,
        }
    };
    Ok(PropertyEdit {
        object,
        set,
        name,
        action,
    })
}

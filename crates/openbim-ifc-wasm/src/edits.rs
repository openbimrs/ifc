//! Moves property edits (#123) from JavaScript objects into the core.
//!
//! An edit is `{ object, set, name, value, setType? }` to write, or
//! `{ object, set, name, remove: true }` to remove; `object` is a `bigint`
//! (or a safe-integer `number`, as for a `ref` id). Every malformed edit is
//! an `invalid-value` error naming its position.

use js_sys::{Array, Reflect};
use openbim_ifc_binding_core::property_edit::{EditAction, PropertyEdit};
use openbim_ifc_binding_core::BindingError;
use wasm_bindgen::JsValue;

use crate::value::{big_i64, from_js};

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// A `PropertyEdit[]`.
pub(crate) fn edits_from_js(value: &JsValue) -> Result<Vec<PropertyEdit>, BindingError> {
    if !Array::is_array(value) {
        return Err(invalid("property edits must be an array"));
    }
    Array::from(value)
        .iter()
        .enumerate()
        .map(|(index, edit)| {
            edit_from_js(&edit).map_err(|error| match error {
                BindingError::InvalidValue(detail) => invalid(format!("edit {index}: {detail}")),
                other => other,
            })
        })
        .collect()
}

fn edit_from_js(edit: &JsValue) -> Result<PropertyEdit, BindingError> {
    if !edit.is_object() {
        return Err(invalid("an edit must be an object"));
    }
    let field = |key: &str| {
        Reflect::get(edit, &key.into()).map_err(|_| invalid(format!("cannot read `{key}`")))
    };
    let text = |key: &str| {
        field(key)?
            .as_string()
            .ok_or_else(|| invalid(format!("`{key}` must be a string")))
    };
    let id = big_i64(&field("object")?, "`object`")?;
    let object = u64::try_from(id).map_err(|_| invalid("`object` must be positive"))?;
    let (set, name) = (text("set")?, text("name")?);
    let remove = field("remove")?;
    let remove = if remove.is_undefined() {
        false
    } else {
        remove
            .as_bool()
            .ok_or_else(|| invalid("`remove` must be a boolean"))?
    };
    let value = field("value")?;
    let set_type = field("setType")?;
    let action = if remove {
        if !value.is_undefined() || !set_type.is_undefined() {
            return Err(invalid("a removal takes no `value` or `setType`"));
        }
        EditAction::Remove
    } else {
        if value.is_undefined() {
            return Err(invalid("a write needs a `value`; a removal `remove: true`"));
        }
        let set_type = if set_type.is_undefined() {
            None
        } else {
            Some(
                set_type
                    .as_string()
                    .ok_or_else(|| invalid("`setType` must be a string"))?,
            )
        };
        EditAction::Set {
            value: from_js(&value)?,
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

//! Moves authoring operations (#330) from Python dicts into the core.
//!
//! The pure-Python `openbim_ifc.AuthorOp` sends one dict per operation:
//! `op` names it, every other key is a field, converted by the kind the
//! core's table (`authoring::OPS`) gives it into the tape form the core
//! reads, so Python cannot interpret an operation differently from C or
//! JavaScript. Ids and handles are `int`s, points and directions three
//! numbers, attributes a dict of tagged-value dicts by name. `None` is an
//! absent field. Every malformed operation is an `invalid-value` error
//! naming its position.

use openbim_ifc_binding_core::authoring::{op_spec, FieldKind};
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{AuthorOp, BindingError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyString};

use crate::convert::from_py;

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// A sequence of operation dicts.
pub fn ops_from_py(value: &Bound<'_, PyAny>) -> Result<Vec<AuthorOp>, BindingError> {
    value
        .try_iter()
        .map_err(|_| invalid("operations must be iterable"))?
        .enumerate()
        .map(|(index, item)| {
            let item = item.map_err(|_| invalid(format!("op {index}: cannot be read")))?;
            op_from_py(&item).map_err(|error| match error {
                BindingError::InvalidValue(detail) => invalid(format!("op {index}: {detail}")),
                other => other,
            })
        })
        .collect()
}

fn op_from_py(item: &Bound<'_, PyAny>) -> Result<AuthorOp, BindingError> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| invalid("an operation must be a dict"))?;
    let name = dict
        .get_item("op")
        .ok()
        .flatten()
        .and_then(|value| value.cast::<PyString>().ok().map(ToString::to_string))
        .ok_or_else(|| invalid("`op` must be a str naming the operation"))?;
    let spec = op_spec(&name).ok_or_else(|| invalid(format!("no operation `{name}`")))?;
    let mut tape = vec![Tagged::Enum(spec.name.to_owned())];
    for (key, value) in dict.iter() {
        let key = key
            .cast::<PyString>()
            .map(ToString::to_string)
            .map_err(|_| invalid("field names must be str"))?;
        if key == "op" || value.is_none() {
            continue;
        }
        let field = spec
            .field(&key)
            .ok_or_else(|| invalid(format!("`{name}` has no field `{key}`")))?;
        let converted = field_value(field.kind, &value)
            .map_err(|detail| invalid(format!("`{key}` {detail}")))?;
        tape.push(Tagged::Text(field.name.to_owned()));
        tape.push(converted);
    }
    AuthorOp::from_tagged(&Tagged::List(tape))
}

fn integer(value: &Bound<'_, PyAny>) -> Result<i64, String> {
    if !value.is_instance_of::<PyInt>() || value.is_instance_of::<PyBool>() {
        return Err("must be an int".into());
    }
    value
        .extract::<i64>()
        .map_err(|_| "must fit a signed 64-bit integer".into())
}

fn id(value: &Bound<'_, PyAny>) -> Result<Tagged, String> {
    u64::try_from(integer(value)?)
        .map(Tagged::Ref)
        .map_err(|_| "must be a non-negative id".into())
}

fn items<'py>(value: &Bound<'py, PyAny>) -> Result<Vec<Bound<'py, PyAny>>, String> {
    if value.is_instance_of::<PyString>() || value.is_instance_of::<PyDict>() {
        return Err("must be a sequence".into());
    }
    value
        .try_iter()
        .map_err(|_| "must be a sequence".to_owned())?
        .map(|item| item.map_err(|_| "cannot be read".to_owned()))
        .collect()
}

fn field_value(kind: FieldKind, value: &Bound<'_, PyAny>) -> Result<Tagged, String> {
    Ok(match kind {
        FieldKind::Text => Tagged::Text(
            value
                .cast::<PyString>()
                .map(ToString::to_string)
                .map_err(|_| "must be a str")?,
        ),
        FieldKind::Id => id(value)?,
        FieldKind::Ids => Tagged::List(items(value)?.iter().map(id).collect::<Result<_, _>>()?),
        FieldKind::Reals => Tagged::List(
            items(value)?
                .iter()
                .map(|item| {
                    let number = if item.is_instance_of::<PyFloat>() {
                        item.extract::<f64>().ok()
                    } else {
                        #[allow(clippy::cast_precision_loss)]
                        integer(item).ok().map(|i| i as f64)
                    };
                    number
                        .filter(|n| n.is_finite())
                        .map(Tagged::Real)
                        .ok_or_else(|| "must hold finite numbers".to_owned())
                })
                .collect::<Result<_, _>>()?,
        ),
        FieldKind::Integer => Tagged::Integer(integer(value)?),
        FieldKind::Attributes => attributes(value).map_err(|error| error.to_string())?,
        _ => return Err("is of a kind this build does not read".into()),
    })
}

/// A dict of tagged-value dicts by name, as the tape's `(name, value)`
/// pairs, in the dict's order.
pub fn attributes(value: &Bound<'_, PyAny>) -> Result<Tagged, BindingError> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| invalid("attributes must be a dict of values by name"))?;
    dict.iter()
        .map(|(key, value)| {
            let name = key
                .cast::<PyString>()
                .map(ToString::to_string)
                .map_err(|_| invalid("attribute names must be str"))?;
            Ok(Tagged::List(vec![Tagged::Text(name), from_py(&value)?]))
        })
        .collect::<Result<_, _>>()
        .map(Tagged::List)
}

/// The pairs of an attributes dict, for the one-operation `create`.
pub fn attribute_pairs(value: &Bound<'_, PyAny>) -> Result<Vec<(String, Tagged)>, BindingError> {
    let Tagged::List(pairs) = attributes(value)? else {
        return Ok(Vec::new());
    };
    Ok(pairs
        .into_iter()
        .filter_map(|pair| match pair {
            Tagged::List(mut pair) if pair.len() == 2 => {
                let value = pair.pop()?;
                match pair.pop()? {
                    Tagged::Text(name) => Some((name, value)),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect())
}

//! Moves authoring operations (#330) from JavaScript objects into the core.
//!
//! An operation is `{ op, ...fields }`: `op` names it (`"product"`,
//! `"assignType"`, ...) and each field is converted by the kind the core's
//! table (`authoring::OPS`) gives it, into the tape form the core reads, so
//! JavaScript cannot interpret an operation differently from C or Python.
//! Ids and handles are `bigint`s (or safe-integer `number`s), points and
//! directions arrays of three numbers, attributes a plain object of
//! `IfcValue`s by name. `undefined` and `null` are an absent field.
//!
//! JavaScript has entropy the WebAssembly module lacks, so the seed fresh
//! `GlobalId`s derive from is drawn here, from `Math.random`.

use js_sys::{Array, Math, Object, Reflect};
use openbim_ifc_binding_core::authoring::{op_spec, FieldKind};
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{AuthorOp, BindingError};
use wasm_bindgen::JsValue;

use crate::value::{big_i64, from_js};

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// A seed for fresh `GlobalId`s: 128 bits from four `Math.random` draws.
pub(crate) fn seed() -> u128 {
    (0..4).fold(0u128, |seed, _| {
        // `Math.random` is in [0, 1), so the product fits 32 bits.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let draw = (Math::random() * 4_294_967_296.0) as u32;
        (seed << 32) | u128::from(draw)
    })
}

/// An `AuthorOp[]`.
pub(crate) fn ops_from_js(value: &JsValue) -> Result<Vec<AuthorOp>, BindingError> {
    if !Array::is_array(value) {
        return Err(invalid("operations must be an array"));
    }
    Array::from(value)
        .iter()
        .enumerate()
        .map(|(index, op)| {
            op_from_js(&op).map_err(|error| match error {
                BindingError::InvalidValue(detail) => invalid(format!("op {index}: {detail}")),
                other => other,
            })
        })
        .collect()
}

fn op_from_js(op: &JsValue) -> Result<AuthorOp, BindingError> {
    if !op.is_object() || Array::is_array(op) {
        return Err(invalid("an operation must be an object"));
    }
    let name = get(op, "op")?
        .as_string()
        .ok_or_else(|| invalid("`op` must be a string naming the operation"))?;
    let spec = op_spec(&name).ok_or_else(|| invalid(format!("no operation `{name}`")))?;
    let mut tape = vec![Tagged::Enum(spec.name.to_owned())];
    for key in Object::keys(&Object::from(op.clone())).iter() {
        let key = key.as_string().unwrap_or_default();
        if key == "op" {
            continue;
        }
        let field = spec
            .field(&key)
            .ok_or_else(|| invalid(format!("`{name}` has no field `{key}`")))?;
        let value = get(op, &key)?;
        if value.is_undefined() || value.is_null() {
            continue;
        }
        let converted = field_value(field.kind, &value)
            .map_err(|detail| invalid(format!("`{key}` {detail}")))?;
        tape.push(Tagged::Text(field.name.to_owned()));
        tape.push(converted);
    }
    AuthorOp::from_tagged(&Tagged::List(tape))
}

fn get(object: &JsValue, key: &str) -> Result<JsValue, BindingError> {
    Reflect::get(object, &key.into()).map_err(|_| invalid(format!("cannot read `{key}`")))
}

fn field_value(kind: FieldKind, value: &JsValue) -> Result<Tagged, String> {
    let id = |value: &JsValue| -> Result<Tagged, String> {
        let id = big_i64(value, "an id").map_err(|error| error.to_string())?;
        u64::try_from(id)
            .map(Tagged::Ref)
            .map_err(|_| "must be a non-negative id".to_owned())
    };
    let array = |value: &JsValue| -> Result<Array, String> {
        if Array::is_array(value) {
            Ok(Array::from(value))
        } else {
            Err("must be an array".to_owned())
        }
    };
    Ok(match kind {
        FieldKind::Text => Tagged::Text(value.as_string().ok_or("must be a string")?),
        FieldKind::Id => id(value)?,
        FieldKind::Ids => Tagged::List(array(value)?.iter().map(|v| id(&v)).collect::<Result<_, _>>()?),
        FieldKind::Reals => Tagged::List(
            array(value)?
                .iter()
                .map(|v| {
                    v.as_f64()
                        .filter(|n| n.is_finite())
                        .map(Tagged::Real)
                        .ok_or_else(|| "must hold finite numbers".to_owned())
                })
                .collect::<Result<_, _>>()?,
        ),
        FieldKind::Integer => {
            Tagged::Integer(big_i64(value, "an integer").map_err(|error| error.to_string())?)
        }
        FieldKind::Attributes => attributes(value).map_err(|error| error.to_string())?,
        _ => return Err("is of a kind this build does not read".to_owned()),
    })
}

/// A plain object of `IfcValue`s by name, as the tape's `(name, value)`
/// pairs, in the object's key order.
pub(crate) fn attributes(value: &JsValue) -> Result<Tagged, BindingError> {
    if !value.is_object() || Array::is_array(value) {
        return Err(invalid("attributes must be an object of IfcValues by name"));
    }
    let object = Object::from(value.clone());
    Object::keys(&object)
        .iter()
        .map(|key| {
            let name = key.as_string().unwrap_or_default();
            let value = from_js(&get(value, &name)?)?;
            Ok(Tagged::List(vec![Tagged::Text(name), value]))
        })
        .collect::<Result<_, _>>()
        .map(Tagged::List)
}

/// The tape of a one-operation batch: `op` with `fields`.
pub(crate) fn single(op: &str, fields: Vec<(&str, Tagged)>) -> Result<AuthorOp, BindingError> {
    let mut tape = vec![Tagged::Enum(op.to_owned())];
    for (key, value) in fields {
        tape.push(Tagged::Text(key.to_owned()));
        tape.push(value);
    }
    AuthorOp::from_tagged(&Tagged::List(tape))
}

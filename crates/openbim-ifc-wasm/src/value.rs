//! Moves [`Tagged`] values into and out of JavaScript objects.
//!
//! All validation of the encoding lives here or in [`Tagged::into_value`]:
//! a malformed JS object is an [`BindingError::InvalidValue`], never a panic
//! and never a silently coerced value.

use js_sys::{Array, BigInt, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

use openbim_ifc_binding_core::value::{Kind, Tagged, MAX_NESTING};
use openbim_ifc_binding_core::BindingError;

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

fn set(object: &Object, key: &str, value: &JsValue) {
    // Setting a plain property on a fresh plain object cannot fail.
    let _ = Reflect::set(object, &key.into(), value);
}

fn get(object: &JsValue, key: &str) -> Result<JsValue, BindingError> {
    Reflect::get(object, &key.into()).map_err(|_| invalid(format!("cannot read `{key}`")))
}

/// Encode a value as a plain JS object.
pub(crate) fn to_js(value: &Tagged) -> JsValue {
    let object = Object::new();
    set(&object, "kind", &value.kind().as_str().into());
    match value {
        Tagged::Null | Tagged::Derived | Tagged::Unknown => {}
        Tagged::Bool(b) => set(&object, "value", &(*b).into()),
        Tagged::Integer(i) => set(&object, "value", &BigInt::from(*i).into()),
        Tagged::Real(r) => set(&object, "value", &(*r).into()),
        Tagged::Text(s) | Tagged::Binary(s) | Tagged::Enum(s) => {
            set(&object, "value", &s.as_str().into());
        }
        Tagged::Ref(id) => set(&object, "id", &BigInt::from(*id).into()),
        Tagged::List(items) => {
            let array: Array = items.iter().map(to_js).collect();
            set(&object, "items", &array.into());
        }
        Tagged::Typed { type_name, value } => {
            set(&object, "type", &type_name.as_str().into());
            set(&object, "value", &to_js(value));
        }
    }
    object.into()
}

/// Decode a JS object, rejecting anything outside the encoding.
pub(crate) fn from_js(value: &JsValue) -> Result<Tagged, BindingError> {
    from_js_at(value, 0)
}

fn from_js_at(value: &JsValue, depth: usize) -> Result<Tagged, BindingError> {
    if depth > MAX_NESTING {
        return Err(invalid(format!("nesting deeper than {MAX_NESTING}")));
    }
    if !value.is_object() {
        return Err(invalid("expected an object with a `kind` field"));
    }
    let kind = get(value, "kind")?
        .as_string()
        .ok_or_else(|| invalid("`kind` must be a string"))?;
    Ok(match Kind::parse(&kind)? {
        Kind::Null => Tagged::Null,
        Kind::Derived => Tagged::Derived,
        Kind::Unknown => Tagged::Unknown,
        Kind::Bool => Tagged::Bool(
            get(value, "value")?
                .as_bool()
                .ok_or_else(|| invalid("bool `value` must be a boolean"))?,
        ),
        Kind::Integer => Tagged::Integer(big_i64(&get(value, "value")?, "integer `value`")?),
        Kind::Real => Tagged::Real(real(&get(value, "value")?)?),
        Kind::Text => Tagged::Text(string(value, "value")?),
        Kind::Binary => Tagged::Binary(string(value, "value")?),
        Kind::Enum => Tagged::Enum(string(value, "value")?),
        Kind::Ref => {
            let id = big_i64(&get(value, "id")?, "ref `id`")?;
            Tagged::Ref(u64::try_from(id).map_err(|_| invalid("ref `id` must be positive"))?)
        }
        Kind::List => {
            let items = get(value, "items")?;
            if !Array::is_array(&items) {
                return Err(invalid("list `items` must be an array"));
            }
            Tagged::List(
                Array::from(&items)
                    .iter()
                    .map(|item| from_js_at(&item, depth + 1))
                    .collect::<Result<_, _>>()?,
            )
        }
        Kind::Typed => Tagged::Typed {
            type_name: string(value, "type")?,
            value: Box::new(from_js_at(&get(value, "value")?, depth + 1)?),
        },
    })
}

fn string(object: &JsValue, key: &str) -> Result<String, BindingError> {
    get(object, key)?
        .as_string()
        .ok_or_else(|| invalid(format!("`{key}` must be a string")))
}

/// A real must be a finite JS number: STEP has no NaN or infinity.
fn real(value: &JsValue) -> Result<f64, BindingError> {
    let number = value
        .as_f64()
        .ok_or_else(|| invalid("real `value` must be a number"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(invalid("real `value` must be finite"))
    }
}

/// A 64-bit integer from a `bigint`, or from a safe-integer `number`.
///
/// Accepting a number is a convenience for small literals; one outside
/// ±(2^53 - 1) may already have lost precision, so it is refused.
fn big_i64(value: &JsValue, what: &str) -> Result<i64, BindingError> {
    if let Some(big) = value.dyn_ref::<BigInt>() {
        return i64::try_from(big.clone())
            .map_err(|_| BindingError::OutOfRange(format!("{what} does not fit in 64 bits")));
    }
    const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
    match value.as_f64() {
        #[allow(clippy::cast_possible_truncation)]
        Some(n) if n.fract() == 0.0 && n.abs() <= MAX_SAFE => Ok(n as i64),
        Some(_) => Err(invalid(format!(
            "{what} must be a bigint, or an integral number within 2^53"
        ))),
        None => Err(invalid(format!("{what} must be a bigint"))),
    }
}

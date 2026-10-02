//! The scalar lexical contract, owned in both directions.
//!
//! XML attribute values are untyped strings, so the reader promotes an
//! attribute's text to a value kind by inference ([`infer`]). The writer may
//! use an attribute only when that inference gives the *same* value back, and
//! [`attribute_text`] is defined as exactly that check: it formats the value
//! and re-runs [`infer`] on the result. The writer's choice is therefore a
//! function of the reader's rule rather than a hand-maintained mirror of it,
//! and a string such as `"0.1"`, `"i7"` or `" i7"` that inference would
//! promote can never be written where it would change kind.
//!
//! Everything inference cannot reproduce is written as a child element whose
//! `kind` attribute names the value kind explicitly. [`element_form`] and
//! [`decode_element`] are that second, explicit pair, and they live here too
//! so every scalar lexical form has one owner.

use crate::error::XmlError;
use ifc_model::{EntityId, Value};

/// Infer the kind of an attribute-encoded scalar.
///
/// Only unambiguous forms are promoted: `i<n>` is a reference, a valid `i64`
/// literal is an integer, and a finite real literal carrying `.` or an
/// exponent is a real. Everything else, including `inf`, `NaN` and an
/// out-of-range literal such as `1e999`, stays text: a real read from an
/// attribute is never non-finite, matching the explicit `kind="real"` path.
pub(crate) fn infer(text: &str) -> Value {
    if let Some(id) = parse_ref(text) {
        return Value::Ref(id);
    }
    if let Ok(integer) = text.parse::<i64>() {
        return Value::Integer(integer);
    }
    if looks_real(text) {
        if let Ok(real) = text.parse::<f64>() {
            if real.is_finite() {
                return Value::Real(real);
            }
        }
    }
    Value::Text(text.into())
}

/// A value's plain-attribute text, or `None` when it needs a typed element.
///
/// Returns text only when [`infer`] maps it back to an identical value (reals
/// compared bit for bit), so this is the reader's rule applied, not restated.
pub(crate) fn attribute_text(value: &Value) -> Option<String> {
    let text = match value {
        Value::Text(text) => text.to_string(),
        Value::Integer(integer) => integer.to_string(),
        Value::Real(real) => format_real(*real),
        Value::Ref(id) => format_ref(*id),
        // No attribute lexical form: these need the element's `kind`.
        Value::Null | Value::Derived | Value::Enum(_) | Value::Binary(_) => return None,
        Value::Bool(_) | Value::LogicalUnknown | Value::List(_) | Value::Typed { .. } => {
            return None;
        }
    };
    same_scalar(&infer(&text), value).then_some(text)
}

/// The explicit `kind` and text of a leaf value written as a child element.
///
/// `None` for the structural forms the writer spells itself: null, derived,
/// list and typed wrapper.
pub(crate) fn element_form(value: &Value) -> Option<(&'static str, String)> {
    let form = match value {
        Value::Text(text) => ("string", text.to_string()),
        Value::Integer(integer) => ("integer", integer.to_string()),
        Value::Real(real) => ("real", format_real(*real)),
        Value::Ref(id) => ("ref", format_ref(*id)),
        Value::Enum(text) => ("enum", text.to_string()),
        Value::Binary(text) => ("binary", text.to_string()),
        Value::Bool(true) => ("logical", "true".into()),
        Value::Bool(false) => ("logical", "false".into()),
        Value::LogicalUnknown => ("logical", "unknown".into()),
        Value::Null | Value::Derived | Value::List(_) | Value::Typed { .. } => return None,
    };
    Some(form)
}

/// Decode a leaf element's text by its explicit `kind`.
///
/// `Ok(None)` means the kind is structural (`list`, `typed`) and handled by
/// the reader. An absent kind is a string. An explicit kind whose text is not
/// a value of that kind is an error, never a coercion.
pub(crate) fn decode_element(kind: &str, text: &str) -> Result<Option<Value>, XmlError> {
    let value = match kind {
        "list" | "typed" => return Ok(None),
        "string" | "" => Value::Text(text.into()),
        "enum" => Value::Enum(text.into()),
        "binary" => Value::Binary(text.into()),
        "logical" => match text {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "unknown" => Value::LogicalUnknown,
            _ => return Err(invalid_scalar("logical", text)),
        },
        "integer" => Value::Integer(text.parse().map_err(|_| invalid_scalar("integer", text))?),
        "real" => {
            let real: f64 = text.parse().map_err(|_| invalid_scalar("real", text))?;
            if !real.is_finite() {
                return Err(invalid_scalar("real", text));
            }
            Value::Real(real)
        }
        "ref" => Value::Ref(parse_ref(text).ok_or_else(|| invalid_scalar("ref", text))?),
        kind => return Err(XmlError::UnknownKind(kind.into())),
    };
    Ok(Some(value))
}

/// `i42` -> `Some(EntityId(42))`; surrounding whitespace is tolerated.
pub(crate) fn parse_ref(text: &str) -> Option<EntityId> {
    let number: u64 = text.trim().strip_prefix('i')?.parse().ok()?;
    Some(EntityId(number))
}

/// The `i<n>` lexical form of an entity reference or id.
pub(crate) fn format_ref(id: EntityId) -> String {
    format!("i{}", id.0)
}

/// Format a real so it survives a round-trip.
///
/// XML has no real/integer distinction, so `1.0` must not be written as `1` --
/// re-reading would infer an integer and change the value's kind. A decimal
/// point or exponent is always present.
pub(crate) fn format_real(real: f64) -> String {
    if real == real.trunc() && real.is_finite() && real.abs() < 1e15 {
        format!("{real:.1}")
    } else {
        let text = format!("{real}");
        if looks_real(&text) {
            text
        } else {
            format!("{text}.0")
        }
    }
}

fn invalid_scalar(kind: &str, value: &str) -> XmlError {
    XmlError::InvalidScalar {
        kind: kind.into(),
        value: value.into(),
    }
}

/// A STEP real always carries `.` or an exponent, which is what distinguishes
/// `1.` from the integer `1`.
fn looks_real(text: &str) -> bool {
    text.contains('.') || text.contains('e') || text.contains('E')
}

/// Scalar identity with reals compared bit for bit, so `-0.0` and `0.0` differ.
pub(crate) fn same_scalar(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Real(left), Value::Real(right)) => left.to_bits() == right.to_bits(),
        (left, right) => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_text_is_refused_exactly_where_inference_changes_the_value() {
        for text in ["0.1", "42", "i7", " i7", "1e5", "+5", "-0"] {
            assert_eq!(attribute_text(&Value::Text(text.into())), None, "{text:?}");
        }
        for text in ["", "inf", "NaN", "1e999", "i", "i-1", "0x10", "plain"] {
            assert_eq!(
                attribute_text(&Value::Text(text.into())).as_deref(),
                Some(text),
                "{text:?}"
            );
        }
    }

    #[test]
    fn inference_never_yields_a_non_finite_real() {
        for text in ["1e999", "-1e999", "inf", "-inf", "NaN", "infinity"] {
            assert_eq!(infer(text), Value::Text(text.into()), "{text:?}");
        }
    }

    #[test]
    fn logical_elements_reject_unrecognised_text() {
        assert!(decode_element("logical", "maybe").is_err());
        assert_eq!(
            decode_element("logical", "unknown").unwrap(),
            Some(Value::LogicalUnknown)
        );
    }
}

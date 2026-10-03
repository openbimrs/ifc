//! Text typed by a declared shape, and values checked against one.

use super::{invalid, type_shape, unsupported, Leaf, Lexical, Shape, MAX_DEPTH};
use crate::error::XmlError;
use ifc_model::Value;
use ifc_schema::{Schema, TypeKind};
use std::sync::Arc;

/// Type one scalar's text by its declared leaf.
///
/// Entity and SELECT leaves have no text form in the XSD layout; in the
/// native layout they take an `i<n>` reference.
pub(crate) fn scalar(leaf: &Leaf, text: &str, lexical: Lexical) -> Result<Value, XmlError> {
    let token = text.trim();
    let value = match leaf {
        Leaf::Integer => Value::Integer(token.parse().map_err(|_| invalid(leaf, text))?),
        Leaf::Real => Value::Real(finite(leaf, token, text)?),
        Leaf::Number => match lexical {
            Lexical::Xsd => Value::Real(finite(leaf, token, text)?),
            Lexical::Native => match token.parse::<i64>() {
                Ok(integer) => Value::Integer(integer),
                Err(_) => Value::Real(finite(leaf, token, text)?),
            },
        },
        Leaf::Text { fixed, .. } => {
            if let Some(width) = fixed {
                if text.chars().count() != *width {
                    return Err(invalid(leaf, text));
                }
            }
            Value::Text(text.into())
        }
        Leaf::Boolean => match (token, lexical) {
            ("true", _) | ("1", Lexical::Xsd) => Value::Bool(true),
            ("false", _) | ("0", Lexical::Xsd) => Value::Bool(false),
            _ => return Err(invalid(leaf, text)),
        },
        Leaf::Logical => match token {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "unknown" => Value::LogicalUnknown,
            _ => return Err(invalid(leaf, text)),
        },
        Leaf::Binary => match lexical {
            Lexical::Xsd => Value::Binary(hex_binary(token).ok_or_else(|| invalid(leaf, text))?),
            Lexical::Native => {
                if !is_step_binary(text) {
                    return Err(invalid(leaf, text));
                }
                Value::Binary(text.into())
            }
        },
        Leaf::Enumeration { members, .. } => {
            let member = members
                .iter()
                .find(|member| member.eq_ignore_ascii_case(token))
                .ok_or_else(|| invalid(leaf, text))?;
            Value::Enum(member.to_ascii_uppercase().into())
        }
        Leaf::Entity(_) | Leaf::Select(_) => match lexical {
            Lexical::Native => Value::Ref(crate::scalar::parse_ref(text).ok_or_else(|| {
                XmlError::TypeMismatch {
                    declared: leaf.describe(),
                    found: format!("text {text:?}"),
                }
            })?),
            Lexical::Xsd => {
                return Err(XmlError::TypeMismatch {
                    declared: leaf.describe(),
                    found: format!("text {text:?}"),
                })
            }
        },
    };
    Ok(value)
}

fn finite(leaf: &Leaf, token: &str, text: &str) -> Result<f64, XmlError> {
    // Rust also accepts `inf`, `NaN` and `infinity`; none is a finite real.
    match token.parse::<f64>() {
        Ok(real) if real.is_finite() => Ok(real),
        _ => Err(invalid(leaf, text)),
    }
}

/// `xs:hexBinary` text as the STEP form the model stores: the count of
/// unused leading bits, then the hex digits. Whole bytes have none unused.
pub(crate) fn hex_binary(token: &str) -> Option<Arc<str>> {
    if token.len() % 2 != 0 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("0{}", token.to_ascii_uppercase()).into())
}

/// A STEP binary literal body: a digit `0`-`3`, then hex digits.
fn is_step_binary(text: &str) -> bool {
    let mut bytes = text.bytes();
    matches!(bytes.next(), Some(b'0'..=b'3')) && bytes.all(|byte| byte.is_ascii_hexdigit())
}

/// Type a whitespace-separated list of scalars and nest it by `shape`.
pub(crate) fn list_text(shape: &Shape, text: &str, lexical: Lexical) -> Result<Value, XmlError> {
    let items = text
        .split_ascii_whitespace()
        .map(|token| scalar(&shape.leaf, token, lexical))
        .collect::<Result<Vec<_>, _>>()?;
    nest(shape, items, None)
}

/// Nest a flat item sequence into `shape`'s aggregation levels.
///
/// ISO 10303-28 writes a nested aggregate flat. The inner sizes come from
/// an `arraySize` (every level's size, outermost first) or from fixed
/// declared bounds; without either the nesting is ambiguous and refused.
pub(crate) fn nest(
    shape: &Shape,
    items: Vec<Value>,
    array_size: Option<&[usize]>,
) -> Result<Value, XmlError> {
    let depth = shape.levels.len();
    if depth == 0 {
        return Err(unsupported(
            "a list where the declared type is not an aggregate".into(),
        ));
    }
    let count = items.len();
    let mismatch = || XmlError::TypeMismatch {
        declared: shape.describe(),
        found: format!("{count} flattened items"),
    };
    let sizes: Vec<usize> = match array_size {
        Some(sizes) => {
            if sizes.len() != depth || sizes.iter().product::<usize>() != count {
                return Err(XmlError::TypeMismatch {
                    declared: shape.describe(),
                    found: format!("arraySize {sizes:?} for {count} flattened items"),
                });
            }
            sizes[1..].to_vec()
        }
        None => shape.levels[1..]
            .iter()
            .map(|level| {
                level.fixed.ok_or_else(|| {
                    unsupported(format!(
                        "a flattened {} without arraySize: its inner sizes are not fixed",
                        shape.describe()
                    ))
                })
            })
            .collect::<Result<_, _>>()?,
    };
    chunk(items, &sizes).ok_or_else(mismatch)
}

/// Split `items` into nested lists of the `inner` sizes, or `None` when
/// they do not divide evenly.
fn chunk(items: Vec<Value>, inner: &[usize]) -> Option<Value> {
    let Some((&size, rest)) = inner.split_first() else {
        return Some(Value::List(items));
    };
    let stride: usize = inner.iter().product();
    if size == 0 || stride == 0 || items.len() % stride != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(items.len() / stride);
    let mut items = items.into_iter();
    loop {
        let group: Vec<Value> = items.by_ref().take(stride).collect();
        if group.is_empty() {
            break;
        }
        out.push(chunk(group, rest)?);
    }
    Some(Value::List(out))
}

/// Check a value read with an explicit kind against its declared shape.
///
/// The native layout writes a value's kind explicitly where inference would
/// lose it; a strict read accepts that kind only where the declaration
/// admits it. Never a coercion: a mismatch is an error.
pub(crate) fn conform(schema: &Schema, shape: &Shape, value: &Value) -> Result<(), XmlError> {
    conform_at(schema, shape, value, 0)
}

fn conform_at(schema: &Schema, shape: &Shape, value: &Value, depth: usize) -> Result<(), XmlError> {
    if depth > MAX_DEPTH {
        return Err(unsupported("a value nested too deeply".into()));
    }
    let mismatch = |found: String| XmlError::TypeMismatch {
        declared: shape.describe(),
        found,
    };
    match value {
        Value::Null | Value::Derived => return Ok(()),
        Value::List(items) => {
            if shape.levels.is_empty() {
                return Err(mismatch("a list".into()));
            }
            let inner = shape.inner();
            for item in items {
                conform_at(schema, &inner, item, depth + 1)?;
            }
            return Ok(());
        }
        _ if !shape.levels.is_empty() => return Err(mismatch(kind_of(value))),
        _ => {}
    }
    let fits = match (&shape.leaf, value) {
        (Leaf::Entity(_), Value::Ref(_)) => true,
        (Leaf::Select(select), Value::Ref(_)) => select_admits_entities(schema, select),
        (Leaf::Select(select), Value::Typed { type_name, value }) => {
            let Some(definition) = schema.type_def(type_name) else {
                return Err(mismatch(format!("a value typed `{type_name}`")));
            };
            if !schema.accepts_type(select, &definition.name) {
                return Err(XmlError::TypeMismatch {
                    declared: select.to_string(),
                    found: format!("a value typed `{type_name}`"),
                });
            }
            let inner = type_shape(schema, &definition.name)?;
            return conform_at(schema, &inner, value, depth + 1);
        }
        (Leaf::Integer, Value::Integer(_))
        | (Leaf::Real | Leaf::Number, Value::Real(_) | Value::Integer(_))
        | (Leaf::Boolean, Value::Bool(_))
        | (Leaf::Logical, Value::Bool(_) | Value::LogicalUnknown)
        | (Leaf::Binary, Value::Binary(_)) => true,
        (Leaf::Text { fixed, .. }, Value::Text(text)) => {
            if let Some(width) = fixed {
                if text.chars().count() != *width {
                    return Err(invalid(&shape.leaf, text));
                }
            }
            true
        }
        (Leaf::Enumeration { members, .. }, Value::Enum(member)) => {
            if !members
                .iter()
                .any(|declared| declared.eq_ignore_ascii_case(member))
            {
                return Err(invalid(&shape.leaf, member));
            }
            true
        }
        _ => false,
    };
    if fits {
        Ok(())
    } else {
        Err(mismatch(kind_of(value)))
    }
}

/// Whether a SELECT admits an entity at any depth.
fn select_admits_entities(schema: &Schema, select: &str) -> bool {
    let mut pending = vec![select.to_string()];
    let mut seen = std::collections::HashSet::new();
    while let Some(name) = pending.pop() {
        if !seen.insert(name.to_ascii_uppercase()) || seen.len() > 256 {
            continue;
        }
        if schema.entity(&name).is_some() {
            return true;
        }
        if let Some(definition) = schema.type_def(&name) {
            if let TypeKind::Select(members) = &definition.kind {
                pending.extend(members.iter().cloned());
            }
        }
    }
    false
}

/// A value's kind, in words, for a mismatch message.
fn kind_of(value: &Value) -> String {
    match value {
        Value::Null => "an unset value".into(),
        Value::Derived => "a derived value".into(),
        Value::Bool(_) | Value::LogicalUnknown => "a logical".into(),
        Value::Integer(_) => "an integer".into(),
        Value::Real(_) => "a real".into(),
        Value::Text(_) => "a string".into(),
        Value::Binary(_) => "a binary".into(),
        Value::Enum(member) => format!("the enumeration value `{member}`"),
        Value::Ref(_) => "an entity reference".into(),
        Value::List(_) => "a list".into(),
        Value::Typed { type_name, .. } => format!("a value typed `{type_name}`"),
    }
}

/// Every reference in a value, with the entity or SELECT its leaf declares.
///
/// Walks typed SELECT values through their own type, so a reference inside
/// `IfcPropertySetDefinitionSet(...)` is judged against that set's members.
pub(crate) fn references(
    schema: &Schema,
    shape: &Shape,
    value: &Value,
    out: &mut Vec<(ifc_model::EntityId, Arc<str>)>,
) -> Result<(), XmlError> {
    match value {
        Value::Ref(id) => match &shape.leaf {
            Leaf::Entity(name) | Leaf::Select(name) if shape.levels.is_empty() => {
                out.push((*id, name.clone()));
            }
            _ => {
                return Err(XmlError::TypeMismatch {
                    declared: shape.describe(),
                    found: "an entity reference".into(),
                })
            }
        },
        Value::List(items) if !shape.levels.is_empty() => {
            let inner = shape.inner();
            for item in items {
                references(schema, &inner, item, out)?;
            }
        }
        Value::Typed { type_name, value } => {
            if let Some(definition) = schema.type_def(type_name) {
                let inner = type_shape(schema, &definition.name)?;
                references(schema, &inner, value, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

//! Values as the text of one cell, and text read back as a value of a
//! declared type (#472, #474).
//!
//! [`value_text`] writes a [`SelectedValue`] as one line of text: `TRUE`,
//! `FALSE` and `UNKNOWN` for a logical, the shortest decimal that reads
//! back as the same number for a real, `#12` for an entity, items joined
//! with `; ` for a list, `name=value` pairs for a property set, and the
//! caller's null text for `NULL`.
//!
//! Reading text back is typed by the target, never guessed from the text:
//! an attribute by its declared type in the release (`IfcLabel` reads the
//! text verbatim, `IfcInteger` an integer, an enumeration one of its items
//! in any case, an entity `#12`), a property by the type it already states
//! (`IFCLABEL`, `IFCLENGTHMEASURE`). Where neither says which type to
//! write -- a SELECT attribute holding nothing, a property that states no
//! value -- the text names it: `IFCLABEL(F90)`, `IfcLengthMeasure(2.5)`.
//! A list or aggregate is not written from text.

use ifc_model::Value;
use ifc_schema::Schema;

use super::value::SelectedValue;
#[cfg(feature = "properties")]
use crate::declared_type::declared_type;
use crate::declared_type::{DeclaredType, SimpleType};

/// `value` as the text of one cell; `NULL` is `null`.
///
/// The text reads back as the same value through the writes of
/// [`SelectorIndex::plan_text_edits`](super::SelectorIndex::plan_text_edits)
/// for every scalar: a real is written as the shortest decimal that parses
/// to the same number, a logical as `TRUE`, `FALSE` or `UNKNOWN`.
#[must_use]
pub fn value_text(value: &SelectedValue, null: &str) -> String {
    match value {
        SelectedValue::Null => null.to_owned(),
        SelectedValue::Derived => "*".to_owned(),
        SelectedValue::Bool(true) => "TRUE".to_owned(),
        SelectedValue::Bool(false) => "FALSE".to_owned(),
        SelectedValue::Unknown => "UNKNOWN".to_owned(),
        SelectedValue::Integer(i) => i.to_string(),
        SelectedValue::Real(r) => r.to_string(),
        SelectedValue::Text(text) | SelectedValue::Binary(text) | SelectedValue::Enum(text) => {
            text.clone()
        }
        SelectedValue::Entity(id) => format!("#{}", id.0),
        SelectedValue::List(items) => items
            .iter()
            .map(|item| value_text(item, null))
            .collect::<Vec<_>>()
            .join("; "),
        SelectedValue::Properties(members) => members
            .iter()
            .map(|(name, value)| format!("{name}={}", value_text(value, null)))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

/// `text` as a value of `declared`, the declared type of an attribute,
/// whose current value is `current`. `Err` names what the text should
/// have been.
pub(super) fn parse_declared(
    schema: &Schema,
    declared: &DeclaredType<'_>,
    current: Option<&Value>,
    text: &str,
) -> Result<Value, String> {
    parse(schema, declared, current, text, 0)
}

#[cfg(feature = "properties")]
/// `text` as a property value: typed by `IFCTYPE(...)` when the text names
/// a defined type of the release, else wrapped in `stated`, the type the
/// property already states.
pub(super) fn parse_property(
    schema: &Schema,
    stated: Option<&str>,
    text: &str,
) -> Result<Value, String> {
    if let Some(typed) = explicitly_typed(schema, text) {
        return typed;
    }
    let Some(stated) = stated else {
        return Err("a typed value such as IFCLABEL(text): the property states no type".to_owned());
    };
    let declared = declared_type(schema, stated);
    wrapped(schema, stated, &declared, text)
}

/// `NAME(inner)` where `NAME` is a defined type or enumeration of the
/// release: `None` when the text does not have that form.
#[cfg(feature = "properties")]
fn explicitly_typed(schema: &Schema, text: &str) -> Option<Result<Value, String>> {
    let (name, inner) = split_typed(text)?;
    let declared = declared_type(schema, name);
    if !matches!(
        declared,
        DeclaredType::Defined { .. } | DeclaredType::Enumeration { .. }
    ) {
        return None;
    }
    Some(wrapped(schema, name, &declared, inner))
}

/// `inner` as a value of `declared`, wrapped in its type name.
fn wrapped(
    schema: &Schema,
    name: &str,
    declared: &DeclaredType<'_>,
    inner: &str,
) -> Result<Value, String> {
    let value = match declared {
        DeclaredType::Defined { underlying, .. } => parse(schema, underlying, None, inner, 1)?,
        DeclaredType::Enumeration { .. } => parse(schema, declared, None, inner, 1)?,
        _ => return Err(format!("{name} is no defined type of {}", schema.name())),
    };
    Ok(Value::Typed {
        type_name: declared.name().unwrap_or(name).to_ascii_uppercase().into(),
        value: Box::new(value),
    })
}

/// `NAME(inner)`: an identifier, an opening parenthesis, and a closing one
/// ending the text.
fn split_typed(text: &str) -> Option<(&str, &str)> {
    let open = text.find('(')?;
    let name = &text[..open];
    let inner = text[open + 1..].strip_suffix(')')?;
    let identifier = name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    identifier.then_some((name, inner))
}

fn parse(
    schema: &Schema,
    declared: &DeclaredType<'_>,
    current: Option<&Value>,
    text: &str,
    depth: usize,
) -> Result<Value, String> {
    if depth > 32 {
        return Err("a value whose declared type nests too deeply".to_owned());
    }
    match declared {
        DeclaredType::Defined { underlying, .. } => {
            parse(schema, underlying, current, text, depth + 1)
        }
        DeclaredType::Simple(simple) => simple_value(*simple, text),
        DeclaredType::Enumeration { name, items } => items
            .iter()
            .find(|item| item.eq_ignore_ascii_case(text.trim()))
            .map(|item| Value::Enum(item.to_ascii_uppercase().into()))
            .ok_or_else(|| format!("one of {name}: {}", items.join(", "))),
        DeclaredType::Entity(name) => {
            reference(text).ok_or_else(|| format!("a reference to an {name}, such as #12"))
        }
        DeclaredType::Select { name, members } => select(schema, name, members, current, text),
        DeclaredType::Aggregate { .. } => Err("a list, which is not written from text".to_owned()),
        DeclaredType::Unresolved(token) => Err(format!(
            "a value of {token}, which the release does not resolve"
        )),
    }
}

fn select(
    schema: &Schema,
    name: &str,
    members: &[DeclaredType<'_>],
    current: Option<&Value>,
    text: &str,
) -> Result<Value, String> {
    let typed_hint = || format!("a typed value of the SELECT {name}, such as IFCLABEL(text)");
    if let Some((type_name, inner)) = split_typed(text) {
        if let Some(member) = member_named(members, type_name) {
            return wrapped(schema, type_name, member, inner);
        }
    }
    if let Some(reference) = reference(text) {
        if has_entity(members) {
            return Ok(reference);
        }
    }
    // A SELECT holding a typed value keeps its type.
    if let Some(Value::Typed { type_name, .. }) = current {
        if let Some(member) = member_named(members, type_name) {
            return wrapped(schema, type_name, member, text);
        }
    }
    Err(typed_hint())
}

/// The defined type or enumeration among `members` (nested SELECTs
/// included) named `name`, any case.
fn member_named<'a, 's>(
    members: &'a [DeclaredType<'s>],
    name: &str,
) -> Option<&'a DeclaredType<'s>> {
    members.iter().find_map(|member| match member {
        DeclaredType::Defined { name: own, .. } | DeclaredType::Enumeration { name: own, .. }
            if own.eq_ignore_ascii_case(name) =>
        {
            Some(member)
        }
        DeclaredType::Select { members, .. } => member_named(members, name),
        _ => None,
    })
}

fn has_entity(members: &[DeclaredType<'_>]) -> bool {
    members.iter().any(|member| match member {
        DeclaredType::Entity(_) => true,
        DeclaredType::Select { members, .. } => has_entity(members),
        _ => false,
    })
}

/// `#12` as a reference.
fn reference(text: &str) -> Option<Value> {
    let digits = text.trim().strip_prefix('#')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits
        .parse()
        .ok()
        .map(|id| Value::Ref(ifc_model::EntityId(id)))
}

fn simple_value(simple: SimpleType, text: &str) -> Result<Value, String> {
    let trimmed = text.trim();
    let logical = |unknown: bool| match trimmed.to_ascii_uppercase().as_str() {
        "TRUE" | ".T." => Some(Value::Bool(true)),
        "FALSE" | ".F." => Some(Value::Bool(false)),
        "UNKNOWN" | ".U." if unknown => Some(Value::LogicalUnknown),
        _ => None,
    };
    let real = || {
        trimmed
            .parse::<f64>()
            .ok()
            .filter(|real| real.is_finite())
            .map(Value::Real)
    };
    let value = match simple {
        SimpleType::String => Some(Value::Text(text.into())),
        SimpleType::Integer => trimmed.parse::<i64>().ok().map(Value::Integer),
        SimpleType::Real => real(),
        SimpleType::Number => trimmed
            .parse::<i64>()
            .ok()
            .map(Value::Integer)
            .or_else(real),
        SimpleType::Boolean => logical(false),
        SimpleType::Logical => logical(true),
        SimpleType::Binary => (!trimmed.is_empty()
            && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| Value::Binary(trimmed.to_ascii_uppercase().into())),
    };
    value.ok_or_else(|| {
        match simple {
            SimpleType::String => "text",
            SimpleType::Integer => "an integer",
            SimpleType::Real => "a finite real number",
            SimpleType::Number => "a number",
            SimpleType::Boolean => "TRUE or FALSE",
            SimpleType::Logical => "TRUE, FALSE or UNKNOWN",
            SimpleType::Binary => "hexadecimal digits",
        }
        .to_owned()
    })
}

#[cfg(all(test, feature = "ifc4"))]
mod tests {
    use super::*;
    use crate::attribute_type;

    fn attribute(
        type_name: &str,
        name: &str,
        current: Option<&Value>,
        text: &str,
    ) -> Result<Value, String> {
        let schema = crate::schema::ifc4();
        let (_, declared) = attribute_type(schema, type_name, name).unwrap();
        parse_declared(schema, &declared, current, text)
    }

    #[test]
    fn text_reads_as_the_declared_type() {
        assert_eq!(
            attribute("IFCWALL", "Name", None, " Wall "),
            Ok(Value::Text(" Wall ".into()))
        );
        assert_eq!(
            attribute("IFCWALL", "PredefinedType", None, "movable"),
            Ok(Value::Enum("MOVABLE".into()))
        );
        let refused = attribute("IFCWALL", "PredefinedType", None, "CURVED").unwrap_err();
        assert!(refused.starts_with("one of IfcWallTypeEnum: "), "{refused}");
        assert_eq!(
            attribute("IFCBUILDINGSTOREY", "Elevation", None, "3"),
            Ok(Value::Real(3.0))
        );
        assert!(attribute("IFCBUILDINGSTOREY", "Elevation", None, "NaN").is_err());
        assert_eq!(
            attribute("IFCDOOR", "OverallHeight", None, "2.1"),
            Ok(Value::Real(2.1))
        );
        assert_eq!(
            attribute("IFCWALL", "ObjectPlacement", None, "#12"),
            Ok(Value::Ref(ifc_model::EntityId(12)))
        );
        assert!(attribute("IFCWALL", "ObjectPlacement", None, "12").is_err());
        assert!(attribute("IFCPROJECT", "RepresentationContexts", None, "#1").is_err());
    }

    #[test]
    fn a_select_takes_the_type_the_text_names_or_the_value_holds() {
        // IfcPropertySingleValue.NominalValue : IfcValue.
        let typed = |type_name: &str, value: Value| Value::Typed {
            type_name: type_name.into(),
            value: Box::new(value),
        };
        assert_eq!(
            attribute(
                "IFCPROPERTYSINGLEVALUE",
                "NominalValue",
                None,
                "IfcLengthMeasure(2.5)"
            ),
            Ok(typed("IFCLENGTHMEASURE", Value::Real(2.5)))
        );
        let held = typed("IFCLABEL", Value::Text("F90".into()));
        assert_eq!(
            attribute(
                "IFCPROPERTYSINGLEVALUE",
                "NominalValue",
                Some(&held),
                "F120"
            ),
            Ok(typed("IFCLABEL", Value::Text("F120".into())))
        );
        let refused =
            attribute("IFCPROPERTYSINGLEVALUE", "NominalValue", None, "F120").unwrap_err();
        assert!(refused.contains("IFCLABEL(text)"), "{refused}");
    }

    #[cfg(feature = "properties")]
    #[test]
    fn a_property_takes_its_stated_type_or_a_named_one() {
        let schema = crate::schema::ifc4();
        assert_eq!(
            parse_property(schema, Some("IFCBOOLEAN"), "true"),
            Ok(Value::Typed {
                type_name: "IFCBOOLEAN".into(),
                value: Box::new(Value::Bool(true))
            })
        );
        assert_eq!(
            parse_property(schema, None, "IFCINTEGER(4)"),
            Ok(Value::Typed {
                type_name: "IFCINTEGER".into(),
                value: Box::new(Value::Integer(4))
            })
        );
        assert!(parse_property(schema, Some("IFCINTEGER"), "4.5").is_err());
        assert!(parse_property(schema, None, "4").is_err());
        // Not a type of the release: the stated type reads it verbatim.
        assert_eq!(
            parse_property(schema, Some("IFCLABEL"), "Rating(F90)"),
            Ok(Value::Typed {
                type_name: "IFCLABEL".into(),
                value: Box::new(Value::Text("Rating(F90)".into()))
            })
        );
    }

    #[test]
    fn values_are_written_as_text_that_reads_back() {
        for real in [0.1, 2.5, 1e-7, -0.0, 1e21, 0.300_000_000_000_000_04] {
            let text = value_text(&SelectedValue::Real(real), "");
            assert_eq!(
                text.parse::<f64>().unwrap().to_bits(),
                real.to_bits(),
                "{text}"
            );
        }
        assert_eq!(value_text(&SelectedValue::Bool(true), ""), "TRUE");
        assert_eq!(value_text(&SelectedValue::Unknown, ""), "UNKNOWN");
        assert_eq!(value_text(&SelectedValue::Null, "-"), "-");
        assert_eq!(
            value_text(
                &SelectedValue::List(vec![SelectedValue::Integer(1), SelectedValue::Null]),
                "$"
            ),
            "1; $"
        );
    }
}

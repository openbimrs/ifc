//! One resolved property value as text (table, CSV) and as JSON.
//!
//! The text form is for reading: a scalar as written, a list in brackets,
//! a bound as `lower..upper`. The JSON form keeps every declared type, so
//! nothing the resolver distinguished is merged.

use ifc::properties::{ExactLogical, ExactTypedValue, ExactValue};
use ifc::EntityId;

use crate::json::Json;

/// The value kind, as the JSON names it.
pub(super) fn kind(value: &ExactValue) -> &'static str {
    match value {
        ExactValue::Enumerated(_) => "enumerated",
        ExactValue::List(_) => "list",
        ExactValue::Bounded(_) => "bounded",
        ExactValue::Table(_) => "table",
        ExactValue::Reference(_) => "reference",
        ExactValue::Complex(_) => "complex",
        _ => "value",
    }
}

/// The value as one line of text; empty for an unset value.
pub(super) fn text(value: &ExactValue) -> String {
    let typed = |values: &[ExactTypedValue]| {
        values
            .iter()
            .map(|value| text(&value.value))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let bound = |value: &Option<ExactTypedValue>| {
        value
            .as_ref()
            .map_or_else(String::new, |value| text(&value.value))
    };
    match value {
        ExactValue::Null => String::new(),
        ExactValue::Bool(value) => value.to_string(),
        ExactValue::Logical(ExactLogical::True) => "true".to_owned(),
        ExactValue::Logical(ExactLogical::False) => "false".to_owned(),
        ExactValue::Logical(ExactLogical::Unknown) => "unknown".to_owned(),
        ExactValue::Binary(digits) => digits.to_string(),
        ExactValue::Integer(value) => value.to_string(),
        ExactValue::Real(value) => value.to_string(),
        ExactValue::Text(text) | ExactValue::Enum(text) => text.to_string(),
        ExactValue::Entity(entity) => format!("#{}", entity.id.0),
        ExactValue::Enumerated(enumerated) => typed(&enumerated.values),
        ExactValue::List(values) => format!("[{}]", typed(values)),
        ExactValue::Bounded(bounded) => {
            let mut shown = format!("{}..{}", bound(&bounded.lower), bound(&bounded.upper));
            if let Some(set_point) = &bounded.set_point {
                shown.push_str(&format!(" (set point {})", text(&set_point.value)));
            }
            shown
        }
        ExactValue::Table(table) => format!(
            "table of {} row{}",
            table.rows.len(),
            if table.rows.len() == 1 { "" } else { "s" }
        ),
        ExactValue::Reference(reference) => reference
            .target
            .as_ref()
            .map_or_else(String::new, |target| format!("#{}", target.id.0)),
        ExactValue::Complex(complex) => format!(
            "complex of {} member{}",
            complex.members.len(),
            if complex.members.len() == 1 { "" } else { "s" }
        ),
        // A value form added to the resolver later: shown, not dropped.
        other => format!("{other:?}"),
    }
}

/// The value as JSON: scalars as JSON scalars, composites as objects whose
/// members carry their declared types.
pub(super) fn json(value: &ExactValue) -> Json {
    let typed = |value: &ExactTypedValue| {
        Json::object([
            ("type", Json::str(value.value_type.to_string())),
            ("value", json(&value.value)),
        ])
    };
    let typed_all = |values: &[ExactTypedValue]| Json::Array(values.iter().map(typed).collect());
    let opt_typed = |value: &Option<ExactTypedValue>| value.as_ref().map_or(Json::Null, typed);
    let id = |id: Option<EntityId>| Json::opt_uint(id.map(|id| id.0));
    let opt_text = |text: &Option<std::sync::Arc<str>>| Json::opt_str(text.as_deref());
    match value {
        ExactValue::Null => Json::Null,
        ExactValue::Bool(value) => Json::Bool(*value),
        ExactValue::Logical(ExactLogical::True) => Json::Bool(true),
        ExactValue::Logical(ExactLogical::False) => Json::Bool(false),
        ExactValue::Logical(ExactLogical::Unknown) => Json::str("UNKNOWN"),
        ExactValue::Integer(value) => Json::Int(*value),
        ExactValue::Real(value) => Json::Real(*value),
        ExactValue::Binary(text) | ExactValue::Text(text) | ExactValue::Enum(text) => {
            Json::str(text.to_string())
        }
        ExactValue::Entity(entity) => Json::object([("ref", Json::uint(entity.id.0))]),
        ExactValue::Enumerated(enumerated) => Json::object([
            ("selected", typed_all(&enumerated.values)),
            (
                "enumeration",
                enumerated.enumeration.as_ref().map_or(Json::Null, |e| {
                    Json::object([
                        ("id", Json::uint(e.id.0)),
                        ("name", Json::str(e.name.to_string())),
                        ("values", typed_all(&e.values)),
                    ])
                }),
            ),
        ]),
        ExactValue::List(values) => typed_all(values),
        ExactValue::Bounded(bounded) => Json::object([
            ("lower", opt_typed(&bounded.lower)),
            ("upper", opt_typed(&bounded.upper)),
            ("set_point", opt_typed(&bounded.set_point)),
        ]),
        ExactValue::Table(table) => Json::object([
            (
                "rows",
                Json::Array(
                    table
                        .rows
                        .iter()
                        .map(|row| Json::Array(vec![typed(&row.defining), typed(&row.defined)]))
                        .collect(),
                ),
            ),
            ("expression", opt_text(&table.expression)),
            ("defining_unit", id(table.defining_unit)),
            ("defined_unit", id(table.defined_unit)),
            ("interpolation", opt_text(&table.interpolation)),
        ]),
        ExactValue::Reference(reference) => Json::object([
            ("usage", opt_text(&reference.usage_name)),
            (
                "target",
                id(reference.target.as_ref().map(|target| target.id)),
            ),
        ]),
        ExactValue::Complex(complex) => Json::object([
            ("usage", opt_text(&complex.usage)),
            ("discrimination", opt_text(&complex.discrimination)),
            ("quality", opt_text(&complex.quality)),
            (
                "members",
                Json::Array(
                    complex
                        .members
                        .iter()
                        .map(|member| {
                            Json::object([
                                ("id", Json::uint(member.id.0)),
                                ("name", Json::str(member.name.to_string())),
                                ("kind", Json::str(kind(&member.value))),
                                ("value_type", Json::opt_str(member.value_type.as_deref())),
                                ("unit", id(member.unit_id)),
                                ("value", json(&member.value)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
        other => Json::str(format!("{other:?}")),
    }
}

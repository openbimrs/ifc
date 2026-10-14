//! Exact property values as [`SelectedValue`]s.
#![cfg(feature = "properties")]

use ifc_model::EntityId;
use ifc_properties::{ExactLogical, ExactTypedValue, ExactValue};

use super::error::SelectorError;
use super::value::SelectedValue;

/// `value`, a property of `entity`, as a selector reads it: an enumerated
/// or list value as the list of its values, a bounded value and a complex
/// property as named members, a reference value as its target. A table
/// value is refused: it maps values to values and has no one value to
/// compare.
pub(super) fn selected(
    entity: EntityId,
    value: &ExactValue,
) -> Result<SelectedValue, SelectorError> {
    Ok(match value {
        ExactValue::Null => SelectedValue::Null,
        ExactValue::Bool(b) => SelectedValue::Bool(*b),
        ExactValue::Logical(ExactLogical::True) => SelectedValue::Bool(true),
        ExactValue::Logical(ExactLogical::False) => SelectedValue::Bool(false),
        ExactValue::Logical(_) => SelectedValue::Unknown,
        ExactValue::Binary(digits) => SelectedValue::Binary(digits.to_string()),
        ExactValue::Integer(i) => SelectedValue::Integer(*i),
        ExactValue::Real(r) => SelectedValue::Real(*r),
        ExactValue::Text(text) => SelectedValue::Text(text.to_string()),
        ExactValue::Enum(item) => SelectedValue::Enum(item.to_string()),
        ExactValue::Entity(target) => SelectedValue::Entity(target.id),
        ExactValue::Enumerated(enumerated) => typed_list(entity, &enumerated.values)?,
        ExactValue::List(values) => typed_list(entity, values)?,
        ExactValue::Bounded(bounded) => {
            let mut members = Vec::new();
            for (name, bound) in [
                ("LowerBoundValue", &bounded.lower),
                ("UpperBoundValue", &bounded.upper),
                ("SetPointValue", &bounded.set_point),
            ] {
                let value = match bound {
                    Some(typed) => selected(entity, &typed.value)?,
                    None => SelectedValue::Null,
                };
                members.push((name.to_owned(), value));
            }
            SelectedValue::Properties(members)
        }
        ExactValue::Reference(reference) => reference
            .target
            .as_ref()
            .map_or(SelectedValue::Null, |target| {
                SelectedValue::Entity(target.id)
            }),
        ExactValue::Complex(complex) => {
            let mut members = Vec::new();
            for member in &complex.members {
                members.push((member.name.to_string(), selected(entity, &member.value)?));
            }
            SelectedValue::Properties(members)
        }
        ExactValue::Table(_) => {
            return Err(SelectorError::UnsupportedValue {
                entity,
                form: "table",
            })
        }
        _ => {
            return Err(SelectorError::UnsupportedValue {
                entity,
                form: "property",
            })
        }
    })
}

fn typed_list(
    entity: EntityId,
    values: &[ExactTypedValue],
) -> Result<SelectedValue, SelectorError> {
    values
        .iter()
        .map(|typed| selected(entity, &typed.value))
        .collect::<Result<_, _>>()
        .map(SelectedValue::List)
}

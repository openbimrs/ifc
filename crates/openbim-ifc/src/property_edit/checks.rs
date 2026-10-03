//! The checks a set edit runs that need no planner state: set types that
//! must agree, the catalog template's word on a set and a member, and the
//! form a new property takes when nothing describes it.

use ifc_model::Value;
use ifc_properties::QuantityKind;

use super::edit::{PropertyEditFailure, SetType};
use super::holder::Release;
use super::plan::Form;
use super::template::{Context, MemberTemplate, SetTemplate, TemplateForm};
use super::value::{describe, is_or_aliases, wrapper};

/// Refuse when `stated` is given and is not `set_type`.
pub(super) fn agree(
    set: &str,
    set_type: SetType,
    stated: Option<SetType>,
    by: &str,
) -> Result<(), PropertyEditFailure> {
    match stated {
        Some(stated) if stated != set_type => Err(PropertyEditFailure::WrongSetType(format!(
            "{set} is an {}, but {by} makes it an {}",
            set_type.entity(),
            stated.entity()
        ))),
        _ => Ok(()),
    }
}

/// The set-level template checks: the set's entity and where it may be
/// stated.
pub(super) fn check_set_template(
    template: &SetTemplate,
    set_type: SetType,
    on_type: bool,
) -> Result<(), PropertyEditFailure> {
    if template.quantity != (set_type == SetType::ElementQuantity) {
        return Err(PropertyEditFailure::Template(format!(
            "{} is a {} set in the catalog, not an {}",
            template.name,
            if template.quantity {
                "quantity"
            } else {
                "property"
            },
            set_type.entity()
        )));
    }
    let refused = match template.context {
        Context::TypeOnly if !on_type => Some("only on a type object"),
        Context::OccurrenceOnly if on_type => Some("only on an occurrence"),
        _ => None,
    };
    match refused {
        Some(where_) => Err(PropertyEditFailure::Template(format!(
            "the catalog states {} {where_}",
            template.name
        ))),
        None => Ok(()),
    }
}

/// The form a template member declares, if this writer writes it.
pub(super) fn template_form(
    member: &MemberTemplate,
    set: &str,
    name: &str,
) -> Result<Form, PropertyEditFailure> {
    Ok(match member.form {
        TemplateForm::Single => Form::Single,
        TemplateForm::Enumerated => Form::Enumerated,
        TemplateForm::List => Form::List,
        TemplateForm::Quantity(kind) => Form::Quantity(kind),
        TemplateForm::Other(what) => {
            return Err(PropertyEditFailure::Unsupported(format!(
                "the catalog declares {set}.{name} as {what}, which this writer does not write"
            )))
        }
    })
}

/// The form of a new property no type object or template describes.
pub(super) fn default_form(
    release: Release,
    set_type: SetType,
    value: &Value,
) -> Result<Form, PropertyEditFailure> {
    match set_type {
        SetType::PropertySet => match value {
            Value::List(_) => Err(PropertyEditFailure::Unsupported(
                "a list for a new property is an enumerated or a list value; only a template or the type object can say which"
                    .to_owned(),
            )),
            _ => Ok(Form::Single),
        },
        SetType::ElementQuantity => {
            let measure = wrapper(value).unwrap_or_default();
            [
                QuantityKind::Length,
                QuantityKind::Area,
                QuantityKind::Volume,
                QuantityKind::Count,
                QuantityKind::Weight,
                QuantityKind::Time,
                QuantityKind::Number,
            ]
            .into_iter()
            .find(|kind| {
                kind.measure_type().eq_ignore_ascii_case(measure)
                    && release.schema.entity(kind.type_name()).is_some()
            })
            .map(Form::Quantity)
            .ok_or_else(|| {
                PropertyEditFailure::InvalidValue(format!(
                    "{} is no quantity measure of {:?}",
                    describe(value),
                    release.version
                ))
            })
        }
    }
}

/// A template member's checks of `value` written as `form`. A new member
/// must have the template's form; an existing one of another form is the
/// file's, and is left to it.
pub(super) fn check_template(
    release: Release,
    set: &str,
    name: &str,
    member: &MemberTemplate,
    form: Form,
    value: &Value,
    new: bool,
) -> Result<(), PropertyEditFailure> {
    let template = |detail: String| {
        PropertyEditFailure::Template(format!(
            "{set}.{name} in the {:?} catalog: {detail}",
            release.version
        ))
    };
    let declared = match member.form {
        TemplateForm::Single => Some(Form::Single),
        TemplateForm::Enumerated => Some(Form::Enumerated),
        TemplateForm::List => Some(Form::List),
        TemplateForm::Quantity(kind) => Some(Form::Quantity(kind)),
        TemplateForm::Other(_) => None,
    };
    if declared != Some(form) {
        return if new {
            Err(template(format!(
                "declared as {:?}, written as {form:?}",
                member.form
            )))
        } else {
            Ok(())
        };
    }
    if let Form::Quantity(kind) = form {
        return match wrapper(value) {
            Some(found) if !found.eq_ignore_ascii_case(kind.measure_type()) => Err(template(
                format!("a {} quantity, not an {found}", kind.type_name()),
            )),
            _ => Ok(()),
        };
    }
    let items: Vec<&Value> = match value {
        Value::List(items) => items.iter().collect(),
        Value::Null => Vec::new(),
        other => vec![other],
    };
    if let Some(data_type) = member
        .data_type
        .as_deref()
        .filter(|data_type| release.schema.type_def(data_type).is_some())
    {
        for item in &items {
            if let Some(found) = wrapper(item) {
                if !is_or_aliases(release.schema, found, data_type) {
                    return Err(template(format!("takes an {data_type}, not an {found}")));
                }
            }
        }
    }
    if form == Form::Enumerated && !member.values.is_empty() {
        for item in &items {
            let text = match item.unwrap_typed() {
                Value::Text(text) => Some(&**text),
                _ => None,
            };
            if !text.is_some_and(|text| member.values.iter().any(|allowed| allowed == text)) {
                return Err(template(format!(
                    "{} is not one of {}",
                    describe_payload(item),
                    member.values.join(", ")
                )));
            }
        }
    }
    Ok(())
}

/// Two `IfcValue`s are the same value: one type, one payload.
pub(super) fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (
            Value::Typed {
                type_name: a,
                value: x,
            },
            Value::Typed {
                type_name: b,
                value: y,
            },
        ) => a.eq_ignore_ascii_case(b) && x == y,
        _ => left == right,
    }
}

/// A value's payload, quoted, for a refusal.
pub(super) fn describe_payload(value: &Value) -> String {
    match value.unwrap_typed() {
        Value::Text(text) => format!("'{text}'"),
        other => format!("{other:?}"),
    }
}

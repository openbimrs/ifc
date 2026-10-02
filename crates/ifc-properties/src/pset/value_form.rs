//! The form an authored `IfcValue` is written in.
//!
//! ISO 10303-21:2016 writes a typed parameter (`IFCLENGTHMEASURE(2.5)`)
//! exactly where the declared type is a SELECT (§12.1.8), and the bare value
//! everywhere else (§12.1.6, §12.1.7). `IfcValue` is a SELECT, so a bare
//! `2.5` in its slot cannot say which member it is, and a reader cannot
//! tell an area from a length. `ifc-validate` reports such a file
//! (`type.select.untyped`); the writers here refuse to produce one.
//!
//! In every bundled release (`references/ifc-spec/*/*.exp`) `IfcValue` is
//! `SELECT (IfcMeasureValue, IfcSimpleValue, IfcDerivedMeasureValue)`, and
//! the closure of those three selects names only defined types: 97 in
//! IFC2X3, 108 in IFC4, IFC4X1 and IFC4X2, 109 in IFC4X3 ADD2, and no
//! entity and no enumeration. So, whatever the release:
//!
//! - a literal (number, string, boolean, logical, binary) is the right kind
//!   of value missing its wrapper, refused with
//!   [`PropertyError::ValueForm`] and `typed_required: true`;
//! - a wrapper around another wrapper writes the member bare-wrapped twice:
//!   every member is a defined type whose underlying type is not a SELECT,
//!   so its parameter is written bare, refused with
//!   [`PropertyError::ValueForm`] and `typed_required: false`;
//! - a reference, an aggregate, an enumeration constant, `*`, or a wrapper
//!   around `$` is no `IfcValue` at all, refused with
//!   [`PropertyError::AuthoringInvalid`].
//!
//! Which member a wrapper names is not checked: these writers take no
//! model, so they cannot see the release whose select-list decides it, and
//! `ifc-validate` judges membership against the declared release.

use ifc_model::Value;

use crate::{PropertyError, PropertyResult};

/// The declared type every slot checked here holds.
const IFC_VALUE: &str = "IfcValue";

/// Refuse `value` unless it is an `IfcValue` written as a typed parameter.
///
/// `$` passes: whether the slot may be unset is the caller's rule.
pub(crate) fn require_ifc_value(
    entity: &'static str,
    attribute: &'static str,
    value: &Value,
) -> PropertyResult<()> {
    let form = |typed_required: bool, found: &Value| PropertyError::ValueForm {
        entity,
        attribute,
        declared: IFC_VALUE,
        typed_required,
        found: describe(found),
    };
    let invalid = |found: &Value| PropertyError::AuthoringInvalid {
        entity,
        attribute,
        value: format!("{} is not an {IFC_VALUE}", describe(found)),
    };
    match value {
        Value::Null => Ok(()),
        Value::Integer(_)
        | Value::Real(_)
        | Value::Text(_)
        | Value::Bool(_)
        | Value::LogicalUnknown
        | Value::Binary(_) => Err(form(true, value)),
        Value::Typed { value: inner, .. } => match inner.as_ref() {
            Value::Typed { .. } => Err(form(false, inner)),
            Value::Null | Value::Derived => Err(invalid(value)),
            _ => Ok(()),
        },
        Value::Ref(_) | Value::List(_) | Value::Enum(_) | Value::Derived => Err(invalid(value)),
    }
}

/// [`require_ifc_value`] over each member of a `LIST OF IfcValue`.
pub(crate) fn require_ifc_values(
    entity: &'static str,
    attribute: &'static str,
    values: &[Value],
) -> PropertyResult<()> {
    values
        .iter()
        .try_for_each(|value| require_ifc_value(entity, attribute, value))
}

/// What a value is, in words, for a refusal.
fn describe(value: &Value) -> String {
    match value {
        Value::Null => "unset ($)".to_owned(),
        Value::Derived => "derived (*)".to_owned(),
        Value::Bool(_) => "a bare boolean".to_owned(),
        Value::LogicalUnknown => "a bare logical unknown".to_owned(),
        Value::Integer(_) => "a bare integer".to_owned(),
        Value::Real(_) => "a bare real".to_owned(),
        Value::Text(_) => "a bare string".to_owned(),
        Value::Binary(_) => "a bare binary literal".to_owned(),
        Value::Enum(_) => "an enumeration constant".to_owned(),
        Value::Ref(_) => "an entity reference".to_owned(),
        Value::List(_) => "an aggregate".to_owned(),
        Value::Typed { type_name, value } => {
            format!("a {type_name} wrapper around {}", describe(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::EntityId;

    fn typed(type_name: &str, value: Value) -> Value {
        Value::Typed {
            type_name: type_name.into(),
            value: Box::new(value),
        }
    }

    fn check(value: &Value) -> PropertyResult<()> {
        require_ifc_value("IFCPROPERTYSINGLEVALUE", "NominalValue", value)
    }

    #[test]
    fn a_typed_parameter_and_unset_are_accepted() {
        assert_eq!(check(&Value::Null), Ok(()));
        assert_eq!(check(&typed("IFCLENGTHMEASURE", Value::Real(2.5))), Ok(()));
        assert_eq!(check(&typed("IFCLABEL", Value::Text("x".into()))), Ok(()));
        // IfcComplexNumber is `ARRAY [1:2] OF REAL`: its parameter is a list.
        let complex = typed(
            "IFCCOMPLEXNUMBER",
            Value::List(vec![Value::Real(1.0), Value::Real(0.0)]),
        );
        assert_eq!(check(&complex), Ok(()));
    }

    #[test]
    fn a_bare_literal_needs_its_wrapper() {
        for value in [
            Value::Real(2.5),
            Value::Integer(3),
            Value::Text("x".into()),
            Value::Bool(true),
            Value::LogicalUnknown,
            Value::Binary("0F".into()),
        ] {
            assert!(
                matches!(
                    check(&value),
                    Err(PropertyError::ValueForm {
                        declared: "IfcValue",
                        typed_required: true,
                        attribute: "NominalValue",
                        ..
                    })
                ),
                "{value:?}"
            );
        }
    }

    #[test]
    fn a_wrapped_wrapper_is_the_wrong_form() {
        let nested = typed("IFCLABEL", typed("IFCLABEL", Value::Text("x".into())));
        assert!(matches!(
            check(&nested),
            Err(PropertyError::ValueForm {
                typed_required: false,
                ..
            })
        ));
    }

    #[test]
    fn what_is_no_value_is_invalid() {
        for value in [
            Value::Ref(EntityId(1)),
            Value::List(vec![typed("IFCLABEL", Value::Text("x".into()))]),
            Value::Enum("NOTDEFINED".into()),
            Value::Derived,
            typed("IFCLABEL", Value::Null),
        ] {
            assert!(
                matches!(check(&value), Err(PropertyError::AuthoringInvalid { .. })),
                "{value:?}"
            );
        }
    }

    #[test]
    fn a_list_is_checked_member_by_member() {
        let good = [typed("IFCLABEL", Value::Text("a".into()))];
        assert_eq!(require_ifc_values("E", "A", &good), Ok(()));
        let bad = [good[0].clone(), Value::Text("b".into())];
        assert!(matches!(
            require_ifc_values("E", "A", &bad),
            Err(PropertyError::ValueForm { .. })
        ));
    }
}

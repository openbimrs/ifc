//! Values checked against the defined type declared for their slot.
//!
//! Both what a value is and the form it is written in are judged: Part 21
//! writes a typed parameter exactly where the declared type is a SELECT, and
//! a bare value everywhere else (see `typed`).

use ifc_model::Value;
use ifc_schema::Schema;

use super::aggregate::element_type;
use super::enumeration;
use super::scalar::{describe_value, primitive_of, FixedWidth};
use super::select;
use super::typed::wrapper_fits;

/// How deeply nested aggregates are descended into.
///
/// The deepest aggregate IFC declares is three levels (a list of lists of
/// measures); anything deeper is left unjudged rather than walked without
/// bound, since the nesting depth is under the file's control.
const MAX_NESTING: usize = 8;

/// Why a value does not fit its declared type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mismatch {
    /// The written form is not what the primitive accepts.
    Primitive {
        /// What the schema expects, in words.
        expected: &'static str,
        /// What the file wrote, in words.
        actual: &'static str,
    },
    /// An enumeration constant the schema does not declare.
    EnumMember {
        /// The constant as written.
        member: String,
        /// The members the schema declares.
        declared: Vec<String>,
    },
    /// A typed value whose type is not a member of the declared SELECT.
    SelectMember {
        /// The wrapper type as written.
        written: String,
        /// The SELECT it had to belong to.
        select: String,
    },
    /// A `STRING(n) FIXED` value written at the wrong width.
    FixedWidth {
        /// The width the schema fixes.
        expected: usize,
        /// The width actually written.
        actual: usize,
    },
    /// A value that is not an entity reference, in a slot only an entity
    /// reference can fill: an entity-typed slot, or a SELECT of entities.
    ExpectedReference {
        /// The entity or SELECT the slot declares.
        declared: String,
        /// What the file wrote, in words.
        actual: &'static str,
    },
    /// A typed parameter of the declared type, or of a specialisation of
    /// it, where the declared type is not a SELECT. ISO 10303-21:2016
    /// §12.1.6 writes such a value bare.
    TypedOutsideSelect {
        /// The wrapper type as written.
        written: String,
        /// The non-SELECT type the slot declares.
        declared: String,
    },
    /// A typed parameter whose type is not the declared non-SELECT type:
    /// the wrong form and the wrong type.
    TypedWrongType {
        /// The wrapper type as written.
        written: String,
        /// The non-SELECT type the slot declares.
        declared: String,
    },
    /// A bare value where the declared type is a SELECT. ISO 10303-21:2016
    /// §12.1.8 writes every value of a SELECT that is not an entity instance
    /// as a typed parameter naming its type.
    UntypedSelectValue {
        /// The SELECT the slot declares.
        select: String,
        /// What the file wrote, in words.
        actual: &'static str,
    },
}

/// Checks one value against one declared type name.
///
/// Returns `None` when the value fits, or when the schema gives no basis to
/// judge it. "No basis" is deliberately not a finding: an unrecognized type
/// token means this validator cannot check the slot, and reporting that per
/// value would bury real defects under noise. The unchecked *rules* are
/// counted once, in the where-rule registry.
///
/// Returns the first of [`check_all`]'s mismatches; use that function to
/// see every independent violation in the value.
#[must_use]
pub fn check(schema: &Schema, declared: &str, value: &Value) -> Option<Mismatch> {
    check_all(schema, declared, value).into_iter().next()
}

/// Every independent way one value fails one declared type name, in the
/// order the value is written.
///
/// The members of an aggregate are each checked against its element type,
/// at every nesting level. The form is judged too: a typed parameter outside
/// a SELECT, and a bare value inside one. A wrapper's form and its
/// parameter are independent: `IFCLABEL(12)` in an `IfcLabel` slot is both
/// the wrong form and a number where a string is declared, and both are
/// returned. Identical mismatches are returned once, since a finding cannot
/// tell them apart. Which *entity* a reference points at needs the model,
/// so it is judged by [`crate::structure::wrong_kind_references`] for
/// entity-typed slots and by [`super::attribute_types`] for SELECTs; this
/// function judges only that a reference was written where one may be.
#[must_use]
pub fn check_all(schema: &Schema, declared: &str, value: &Value) -> Vec<Mismatch> {
    let mut found = Vec::new();
    check_nested(schema, declared, value, 0, &mut found);
    found
}

/// Records `mismatch` unless an identical one already is.
fn record(found: &mut Vec<Mismatch>, mismatch: Mismatch) {
    if !found.contains(&mismatch) {
        found.push(mismatch);
    }
}

fn check_nested(
    schema: &Schema,
    declared: &str,
    value: &Value,
    depth: usize,
    found: &mut Vec<Mismatch>,
) {
    let mismatch = match value {
        // `$` and `*` carry no type; presence is `structure`'s concern.
        Value::Null | Value::Derived => None,
        Value::List(items) => {
            if depth >= MAX_NESTING {
                return;
            }
            // Whether a list belongs here at all is
            // `structure::cardinality`'s question; this one is whether its
            // members fit.
            let element = element_type(schema, declared);
            for item in items {
                check_nested(schema, &element, item, depth + 1, found);
            }
            None
        }
        Value::Ref(_) => reference_in_value_slot(schema, declared, value),
        other if requires_reference(schema, declared, other) => Some(Mismatch::ExpectedReference {
            declared: declared.to_string(),
            actual: describe_value(other),
        }),
        other
            if !matches!(other, Value::Typed { .. })
                && select::resolve_select(schema, declared).is_some() =>
        {
            // Every non-entity value of a SELECT names its type (§12.1.8).
            // A SELECT of entities alone was answered above.
            Some(Mismatch::UntypedSelectValue {
                select: declared.to_string(),
                actual: describe_value(other),
            })
        }
        Value::Enum(member) => match enumeration::is_member(schema, declared, member) {
            Some(true) | None => None,
            Some(false) => Some(Mismatch::EnumMember {
                member: member.to_string(),
                declared: enumeration::members(schema, declared)
                    .map(<[String]>::to_vec)
                    .unwrap_or_default(),
            }),
        },
        Value::Typed { type_name, value } => {
            typed(schema, declared, type_name, value, depth, found);
            None
        }
        Value::Text(text) => {
            // A fixed-width string is wrong at any other length, even when
            // it is a perfectly good string. `IfcGloballyUniqueId` is
            // `STRING(22) FIXED`; a 21-character GUID is malformed.
            match FixedWidth::from_resolved(&schema.resolve_defined(declared)) {
                Some(width) if !width.accepts(text) => Some(Mismatch::FixedWidth {
                    expected: width.0,
                    actual: text.chars().count(),
                }),
                _ => primitive_mismatch(schema, declared, value),
            }
        }
        other => primitive_mismatch(schema, declared, other),
    };
    if let Some(mismatch) = mismatch {
        record(found, mismatch);
    }
}

/// A typed parameter `type_name(inner)` written in a `declared` slot.
///
/// In a SELECT slot the wrapper must name a member of the select-list. In
/// any other slot a wrapper is the wrong form (§12.1.6), and of the wrong
/// type too when it does not name the declared type. Either way the
/// parameter is judged as well, against the wrapper's type, which is what
/// it claims to be: a bad parameter is a separate violation, and neither
/// finding masks the other.
fn typed(
    schema: &Schema,
    declared: &str,
    type_name: &str,
    inner: &Value,
    depth: usize,
    found: &mut Vec<Mismatch>,
) {
    if select::resolve_select(schema, declared).is_some() {
        if select::accepts(schema, declared, type_name) == Some(false) {
            record(
                found,
                Mismatch::SelectMember {
                    written: type_name.to_string(),
                    select: declared.to_string(),
                },
            );
        }
        check_nested(schema, type_name, inner, depth, found);
        return;
    }
    match wrapper_fits(schema, type_name, declared) {
        Some(false) => record(
            found,
            Mismatch::TypedWrongType {
                written: type_name.to_string(),
                declared: declared.to_string(),
            },
        ),
        Some(true) => record(
            found,
            Mismatch::TypedOutsideSelect {
                written: type_name.to_string(),
                declared: declared.to_string(),
            },
        ),
        None => {}
    }
    check_nested(schema, type_name, inner, depth, found);
}

/// Whether only an entity reference can fill a `declared` slot, so that
/// `value` is wrong whatever it holds.
///
/// A typed wrapper in a SELECT of entities is left to the SELECT membership
/// check, which names the wrapper; everywhere else a non-reference in an
/// entity-typed slot is reported here.
fn requires_reference(schema: &Schema, declared: &str, value: &Value) -> bool {
    if schema.entity(declared).is_some() {
        return true;
    }
    !matches!(value, Value::Typed { .. })
        && select::admits_only_entities(schema, declared) == Some(true)
}

/// A reference written where the declared type is not an entity.
///
/// Entity and SELECT slots take references; a slot resolving to an EXPRESS
/// primitive does not, and a reference there is a type error however valid
/// its target is.
fn reference_in_value_slot(schema: &Schema, declared: &str, reference: &Value) -> Option<Mismatch> {
    let is_select = select::resolve_select(schema, declared).is_some();
    if schema.entity(declared).is_some() || is_select {
        return None;
    }
    primitive_mismatch(schema, declared, reference)
}

/// The value against the primitive `declared` resolves to, if any.
fn primitive_mismatch(schema: &Schema, declared: &str, value: &Value) -> Option<Mismatch> {
    let primitive = primitive_of(schema, declared)?;
    if primitive.accepts(value) {
        None
    } else {
        Some(Mismatch::Primitive {
            expected: primitive.describe(),
            actual: describe_value(value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::EntityId;

    #[test]
    fn aggregate_members_are_checked_against_the_element_type() {
        let schema = ifc_schema::ifc4();
        let good = Value::List(vec![Value::Real(1.0), Value::Integer(2)]);
        assert_eq!(check(schema, "IfcLengthMeasure", &good), None);
        let bad = Value::List(vec![Value::Real(1.0), Value::Text("x".into())]);
        assert!(matches!(
            check(schema, "IfcLengthMeasure", &bad),
            Some(Mismatch::Primitive { .. })
        ));
        // An aliased aggregate: IfcLineIndex = LIST [2:?] OF IfcPositiveInteger.
        let line = Value::List(vec![Value::Integer(1), Value::Real(2.5)]);
        assert!(matches!(
            check(schema, "IfcLineIndex", &line),
            Some(Mismatch::Primitive { .. })
        ));
    }

    #[test]
    fn a_non_reference_in_an_entity_slot_is_a_mismatch() {
        let schema = ifc_schema::ifc4();
        for value in [
            Value::Text("task".into()),
            Value::Integer(1),
            Value::Enum("NOTDEFINED".into()),
            Value::Typed {
                type_name: "IFCLABEL".into(),
                value: Box::new(Value::Text("task".into())),
            },
        ] {
            assert!(
                matches!(
                    check(schema, "IfcProcess", &value),
                    Some(Mismatch::ExpectedReference { .. })
                ),
                "{value:?}"
            );
        }
        assert_eq!(check(schema, "IfcProcess", &Value::Ref(EntityId(1))), None);
        // A SELECT of entities takes only references, too.
        assert!(matches!(
            check(schema, "IfcActorSelect", &Value::Text("me".into())),
            Some(Mismatch::ExpectedReference { .. })
        ));
        // A SELECT that also takes values needs the typed form instead
        // (ISO 10303-21:2016 §12.1.8), which is a different finding.
        assert!(matches!(
            check(schema, "IfcValue", &Value::Real(1.0)),
            Some(Mismatch::UntypedSelectValue { .. })
        ));
    }

    #[test]
    fn a_reference_in_a_value_slot_is_a_mismatch() {
        let schema = ifc_schema::ifc4();
        assert!(matches!(
            check(schema, "IfcLabel", &Value::Ref(EntityId(1))),
            Some(Mismatch::Primitive { .. })
        ));
        assert_eq!(
            check(schema, "IfcMaterialSelect", &Value::Ref(EntityId(1))),
            None
        );
    }

    /// Each bad member of an aggregate is its own violation, at every
    /// nesting level (#215).
    #[test]
    fn every_bad_member_of_an_aggregate_is_reported() {
        let schema = ifc_schema::ifc4();
        let bad = Value::List(vec![
            Value::Text("x".into()),
            Value::Real(1.0),
            Value::Bool(true),
        ]);
        let found = check_all(schema, "IfcLengthMeasure", &bad);
        assert_eq!(found.len(), 2, "{found:?}");
        // `IfcCartesianPointList3D.CoordList` is `LIST OF LIST OF
        // IfcLengthMeasure`: two rows, each with its own defect.
        let rows = Value::List(vec![
            Value::List(vec![Value::Text("x".into()), Value::Real(0.0)]),
            Value::List(vec![Value::Real(0.0), Value::Bool(false)]),
        ]);
        assert_eq!(check_all(schema, "IfcLengthMeasure", &rows).len(), 2);
        // The same defect twice cannot be told apart in a finding.
        let twice = Value::List(vec![Value::Text("x".into()), Value::Text("y".into())]);
        assert_eq!(check_all(schema, "IfcLengthMeasure", &twice).len(), 1);
        // `check` still answers the first.
        assert!(matches!(
            check(schema, "IfcLengthMeasure", &bad),
            Some(Mismatch::Primitive {
                actual: "a string",
                ..
            })
        ));
    }

    /// A wrapper's form and its parameter are judged independently, so a
    /// bad payload inside a wrapper does not hide the wrong form (#215).
    #[test]
    fn a_wrong_form_and_a_bad_payload_are_both_reported() {
        let schema = ifc_schema::ifc4();
        let wrapped = Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Value::Integer(12)),
        };
        let found = check_all(schema, "IfcLabel", &wrapped);
        assert!(
            found
                .iter()
                .any(|m| matches!(m, Mismatch::TypedOutsideSelect { .. })),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|m| matches!(m, Mismatch::Primitive { .. })),
            "{found:?}"
        );
        // A wrapper outside its SELECT, around a payload its own type
        // refuses: both the membership and the payload.
        let foreign = Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Value::Real(1.0)),
        };
        let found = check_all(schema, "IfcMeasureValue", &foreign);
        assert!(
            found
                .iter()
                .any(|m| matches!(m, Mismatch::SelectMember { .. })),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|m| matches!(m, Mismatch::Primitive { .. })),
            "{found:?}"
        );
    }
}

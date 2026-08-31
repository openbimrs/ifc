//! Values checked against the defined type declared for their slot.

use ifc_model::Value;
use ifc_schema::Schema;

use super::enumeration;
use super::scalar::{describe_value, primitive_of, FixedWidth};
use super::select;

/// Why a value does not fit its declared type.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

/// Checks one value against one declared type name.
///
/// Returns `None` when the value fits, or when the schema gives no basis to
/// judge it. "No basis" is deliberately not a finding: an unrecognized type
/// token means this validator cannot check the slot, and reporting that per
/// value would bury real defects under noise. The unchecked *rules* are
/// counted once, in the where-rule registry.
#[must_use]
pub fn check(schema: &Schema, declared: &str, value: &Value) -> Option<Mismatch> {
    match value {
        // `$` and `*` carry no type; presence is `structure`'s concern.
        Value::Null | Value::Derived => None,
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
            // A typed wrapper in a SELECT slot must name a SELECT member.
            if let Some(false) = select::accepts(schema, declared, type_name) {
                return Some(Mismatch::SelectMember {
                    written: type_name.to_string(),
                    select: declared.to_string(),
                });
            }
            // Otherwise the wrapper names the real type: check the payload
            // against it rather than against the slot's declared type.
            check(schema, type_name, value)
        }
        // A reference's target type is checked structurally, where the model
        // is available to resolve it.
        Value::Ref(_) => None,
        // Aggregate shape is `structure::cardinality`'s concern; element
        // types are not declared by the parser's flat type token.
        Value::List(_) => None,
        Value::Text(text) => {
            // A fixed-width string is wrong at any other length, even when
            // it is a perfectly good string. `IfcGloballyUniqueId` is
            // `STRING(22) FIXED`; a 21-character GUID is malformed.
            if let Some(width) = FixedWidth::from_resolved(&schema.resolve_defined(declared)) {
                if !width.accepts(text) {
                    return Some(Mismatch::FixedWidth {
                        expected: width.0,
                        actual: text.chars().count(),
                    });
                }
            }
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
        other => {
            let primitive = primitive_of(schema, declared)?;
            if primitive.accepts(other) {
                None
            } else {
                Some(Mismatch::Primitive {
                    expected: primitive.describe(),
                    actual: describe_value(other),
                })
            }
        }
    }
}

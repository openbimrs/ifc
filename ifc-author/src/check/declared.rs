//! Does this value fit the attribute's declared EXPRESS type?
//!
//! # Why this is deliberately shallow
//!
//! EXPRESS types form a graph: `IfcPositiveLengthMeasure` is a `REAL`,
//! `IfcLabel` is a `STRING`, and a SELECT admits any of its members, each of
//! which may itself be a defined type or an entity. Chasing that graph to a
//! definitive yes/no answer for every value is a validation problem, and
//! `ifc-validate` owns it.
//!
//! This module answers the narrower, construction-time question: **is the
//! caller obviously wrong?** Passing a string where a real is declared is a
//! programming error worth refusing at the call site. Anything this module
//! cannot resolve is *accepted*, because a builder that rejects valid input is
//! worse than one that misses an exotic mistake -- the model is still audited
//! by `ifc-validate` afterwards.

use ifc_model::Value;
use ifc_schema::{Schema, TypeKind};

/// A short human description of what a value actually is, for error messages.
pub(crate) fn describe_value(value: &Value) -> String {
    match value {
        Value::Null => "unset ($)".to_owned(),
        Value::Derived => "derived (*)".to_owned(),
        Value::Bool(_) => "a boolean".to_owned(),
        Value::LogicalUnknown => "a logical unknown".to_owned(),
        Value::Integer(_) => "an integer".to_owned(),
        Value::Real(_) => "a real".to_owned(),
        Value::Text(_) => "a string".to_owned(),
        Value::Binary(_) => "a binary literal".to_owned(),
        Value::Enum(_) => "an enumeration constant".to_owned(),
        Value::Ref(_) => "an entity reference".to_owned(),
        Value::List(_) => "an aggregate".to_owned(),
        Value::Typed { type_name, .. } => format!("a {type_name} wrapper"),
    }
}

/// The primitive shapes an EXPRESS declaration can bottom out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Number,
    Text,
    Logical,
    Binary,
    /// An entity reference, or a SELECT that admits one.
    Reference,
    /// An enumeration; members are checked by name.
    Enumeration,
    /// Resolvable only at validation time -- accept anything.
    Unresolved,
}

/// Resolve a declared type token to the value shape it admits.
///
/// Follows defined-type aliases up to a bounded depth, because a malformed
/// schema can declare `TYPE A = B; TYPE B = A;` and an unbounded walk would
/// hang the writer.
fn shape_of(schema: &Schema, type_name: &str, depth: u8) -> Shape {
    if depth == 0 {
        return Shape::Unresolved;
    }
    match type_name.to_ascii_uppercase().as_str() {
        "REAL" | "INTEGER" | "NUMBER" => return Shape::Number,
        "STRING" => return Shape::Text,
        "BOOLEAN" | "LOGICAL" => return Shape::Logical,
        "BINARY" => return Shape::Binary,
        _ => {}
    }
    // An entity name is a reference. Checked before defined types because IFC
    // declares both in the same namespace.
    if schema.entity(type_name).is_some() {
        return Shape::Reference;
    }
    let Some(def) = schema.type_def(type_name) else {
        return Shape::Unresolved;
    };
    match &def.kind {
        TypeKind::Defined(alias) => shape_of(schema, alias, depth - 1),
        TypeKind::Enumeration(_) => Shape::Enumeration,
        // A SELECT admits several members. Resolve only when every member
        // agrees on a shape; a mixed select cannot refuse anything.
        TypeKind::Select(members) => {
            let mut shapes = members.iter().map(|m| shape_of(schema, m, depth - 1));
            let Some(first) = shapes.next() else {
                return Shape::Unresolved;
            };
            if shapes.all(|s| s == first) {
                first
            } else {
                Shape::Unresolved
            }
        }
    }
}

/// Whether `value` is admissible for an attribute declared as `type_name`.
///
/// Returns `true` when the declaration cannot be resolved: see the module note
/// on why this is deliberately permissive.
pub(crate) fn value_matches(schema: &Schema, type_name: &str, value: &Value) -> bool {
    // `$` is how an unset optional is written, and `*` how a derived attribute
    // is. Neither carries a type, so neither can mismatch one.
    if matches!(value, Value::Null | Value::Derived) {
        return true;
    }
    // A typed wrapper states its own type (`IFCLENGTHMEASURE(2.5)`). Trust the
    // caller's declaration and check the payload against it, so that a wrapper
    // around a string where a real is wanted is still caught.
    if let Value::Typed {
        type_name: wrapper,
        value: inner,
    } = value
    {
        return value_matches(schema, wrapper, inner);
    }
    match shape_of(schema, type_name, 16) {
        Shape::Unresolved => true,
        Shape::Number => matches!(value, Value::Integer(_) | Value::Real(_)),
        // IFC files are inconsistent about quoting, but a number where a label
        // is declared is a real mistake worth catching.
        Shape::Text => matches!(value, Value::Text(_)),
        Shape::Logical => matches!(value, Value::Bool(_) | Value::LogicalUnknown),
        Shape::Binary => matches!(value, Value::Binary(_)),
        Shape::Reference => matches!(value, Value::Ref(_)),
        Shape::Enumeration => matches!(value, Value::Enum(_)),
    }
}

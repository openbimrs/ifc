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
//!
//! # The form is checked too
//!
//! ISO 10303-21 writes a typed parameter (`IFCAREAMEASURE(12.5)`) exactly
//! where the declared type is a SELECT, and the bare value everywhere else
//! (see `form`). A wrapper in a slot that is not a SELECT is refused, as
//! [`AuthorError::ValueForm`](crate::AuthorError::ValueForm) when it names
//! the declared type and as a type mismatch when it names another type. A
//! bare value where a SELECT is declared is refused too, because the reader
//! cannot tell which member it is. The same permissive rule applies: a
//! declared type the tables cannot resolve accepts either form.
//!
//! # SELECTs are not supertypes
//!
//! A reference is checked for shape only here. Code that must decide whether
//! a referenced entity fits a declared type uses
//! [`Schema::accepts_type`](ifc_schema::Schema::accepts_type), never
//! [`Schema::is_a`](ifc_schema::Schema::is_a): a SELECT such as
//! `IfcAxis2Placement` is not a supertype of its members, so `is_a` rejects
//! them, while `accepts_type` resolves SELECTs and defined-type aliases.

use ifc_model::Value;
use ifc_schema::{Schema, TypeKind};

use super::form::{aliases_to, form_of, select_lists, Form};

/// What [`judge_value`] concludes about one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Admissible, or not resolvable enough to refuse.
    Fits,
    /// A value of another type than the one declared.
    WrongType,
    /// A value of the declared type in the wrong form: bare where the
    /// declared SELECT needs a typed parameter (`typed_required`), or a
    /// typed parameter where the declared type is not a SELECT.
    WrongForm {
        /// Whether the declared type requires the typed form.
        typed_required: bool,
    },
}

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
        // A declaration form the tables record but this check does not know.
        _ => Shape::Unresolved,
    }
}

/// Whether `value` is admissible for an attribute declared as `type_name`,
/// in both type and form.
///
/// Returns [`Verdict::Fits`] when the declaration cannot be resolved: see the
/// module note on why this is deliberately permissive.
pub(crate) fn judge_value(schema: &Schema, type_name: &str, value: &Value) -> Verdict {
    // `$` is how an unset optional is written, and `*` how a derived attribute
    // is. Neither carries a type, so neither can mismatch one.
    if matches!(value, Value::Null | Value::Derived) {
        return Verdict::Fits;
    }
    match (form_of(schema, type_name), value) {
        (Form::Unresolved, _) => Verdict::Fits,
        // A typed parameter in a SELECT slot names the member it is; its
        // parameter is then judged against that member's own type.
        (
            Form::Typed,
            Value::Typed {
                type_name: wrapper,
                value: inner,
            },
        ) => {
            if select_lists(schema, type_name, wrapper) == Some(false) {
                Verdict::WrongType
            } else {
                judge_value(schema, wrapper, inner)
            }
        }
        (Form::Typed, Value::Ref(_) | Value::List(_)) => shape_verdict(schema, type_name, value),
        // A bare value in a SELECT of entities is the wrong type; in any
        // other SELECT it is the right type missing its wrapper.
        (Form::Typed, _) => {
            if shape_of(schema, type_name, 16) == Shape::Reference {
                Verdict::WrongType
            } else {
                Verdict::WrongForm {
                    typed_required: true,
                }
            }
        }
        (
            Form::Bare,
            Value::Typed {
                type_name: wrapper, ..
            },
        ) => {
            if aliases_to(schema, wrapper, type_name) {
                Verdict::WrongForm {
                    typed_required: false,
                }
            } else {
                Verdict::WrongType
            }
        }
        (Form::Bare, _) => shape_verdict(schema, type_name, value),
    }
}

/// The value's shape against the primitive shape the declaration admits.
fn shape_verdict(schema: &Schema, type_name: &str, value: &Value) -> Verdict {
    let fits = match shape_of(schema, type_name, 16) {
        Shape::Unresolved => true,
        Shape::Number => matches!(value, Value::Integer(_) | Value::Real(_)),
        // IFC files are inconsistent about quoting, but a number where a label
        // is declared is a real mistake worth catching.
        Shape::Text => matches!(value, Value::Text(_)),
        Shape::Logical => matches!(value, Value::Bool(_) | Value::LogicalUnknown),
        Shape::Binary => matches!(value, Value::Binary(_)),
        Shape::Reference => matches!(value, Value::Ref(_)),
        Shape::Enumeration => matches!(value, Value::Enum(_)),
    };
    if fits {
        Verdict::Fits
    } else {
        Verdict::WrongType
    }
}

/// The element type of a declared type that aliases an aggregate.
///
/// `TYPE IfcCompoundPlaneAngleMeasure = LIST [3:4] OF INTEGER;` makes an
/// attribute declared `IfcCompoundPlaneAngleMeasure` an aggregate even though
/// the attribute declaration itself has no `LIST`. Follows defined-type
/// aliases with the same bounded depth as [`judge_value`]; `None` when the
/// type is not an aggregate.
pub(crate) fn aggregate_element(schema: &Schema, type_name: &str) -> Option<String> {
    let mut current = type_name.to_owned();
    for _ in 0..16 {
        let TypeKind::Defined(rhs) = &schema.type_def(&current)?.kind else {
            return None;
        };
        let rhs = rhs.trim();
        let keyword = rhs
            .split(|c: char| !c.is_ascii_alphabetic())
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        if matches!(keyword.as_str(), "LIST" | "SET" | "ARRAY" | "BAG") {
            let (_, element) = rhs.split_once(" OF ")?;
            let element = element
                .trim()
                .trim_start_matches("UNIQUE ")
                .trim_start_matches("OPTIONAL ")
                .trim();
            return Some(element.to_owned());
        }
        current = rhs.to_owned();
    }
    None
}

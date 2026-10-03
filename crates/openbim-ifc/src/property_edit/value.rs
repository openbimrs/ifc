//! Whether a value is admissible in the declared release.
//!
//! `IfcPropertySingleValue.NominalValue`, the members of an enumerated or
//! list value, are `IfcValue`, a SELECT whose closure names only defined
//! types in every bundled release. ISO 10303-21 writes such a value as a
//! typed parameter naming a member of the select-list (§12.1.8), and the
//! parameter itself in the member's underlying type. Both are checked here
//! against the release's own table: the wrapper must be a member of that
//! release's `IfcValue` (`IFCPOSITIVERATIOMEASURE` is; `IFCGLOBALLYUNIQUEID`
//! is not), and the payload must be what the member resolves to: a real for
//! `REAL`, an integer for `INTEGER`, either for `NUMBER`, text for `STRING`,
//! a boolean for `BOOLEAN`, a boolean or `.U.` for `LOGICAL`, binary for
//! `BINARY`, and an aggregate of the stated bounds for `ARRAY [1:2] OF
//! REAL` (`IfcComplexNumber`).
//!
//! A quantity's value slot is a defined measure, not a SELECT: the read side
//! carries it typed with that measure, and the writer takes the same form,
//! checks the wrapper is exactly that measure and writes the number bare.

use std::collections::BTreeSet;

use ifc_model::Value;
use ifc_schema::{Schema, TypeKind};

/// How many alias hops and nested aggregates are followed; the bundled
/// tables need three.
const MAX_DEPTH: usize = 16;

/// Exact integers a quantity accepts: a larger one does not survive the
/// writers' `f64`.
const MAX_EXACT_INTEGER: i64 = 1 << 53;

/// Refuse `value` unless it is an `IfcValue` of `schema` in the typed form;
/// `$` passes when `allow_null`.
pub(super) fn check_ifc_value(
    schema: &Schema,
    value: &Value,
    allow_null: bool,
) -> Result<(), String> {
    match value {
        Value::Null if allow_null => Ok(()),
        Value::Null => Err("a value is required here, not $".to_owned()),
        Value::Typed { type_name, value } => {
            if !select_members(schema, "IfcValue").contains(&type_name.to_ascii_uppercase()) {
                return Err(format!("{type_name} is no IfcValue of {}", schema.name()));
            }
            fits(schema, type_name, value, MAX_DEPTH).map_err(|detail| {
                format!(
                    "{type_name}({}) does not hold a value of its type: {detail}",
                    describe(value)
                )
            })
        }
        other => Err(format!(
            "{} is no IfcValue: it needs its typed wrapper, such as IFCLABEL('...')",
            describe(other)
        )),
    }
}

/// The number a quantity of `measure` holds, from its typed form.
pub(super) fn quantity_number(measure: &str, value: &Value) -> Result<f64, String> {
    let Value::Typed { type_name, value } = value else {
        return Err(format!(
            "a quantity takes a typed {measure}, not {}",
            describe(value)
        ));
    };
    if !type_name.eq_ignore_ascii_case(measure) {
        return Err(format!(
            "the quantity holds an {measure}, not an {type_name}"
        ));
    }
    match value.as_ref() {
        Value::Real(number) if number.is_finite() => Ok(*number),
        Value::Integer(number) if number.abs() <= MAX_EXACT_INTEGER => Ok(*number as f64),
        other => Err(format!(
            "{type_name} takes a finite number, not {}",
            describe(other)
        )),
    }
}

/// The wrapper a value names, when it is typed.
pub(super) fn wrapper(value: &Value) -> Option<&str> {
    match value {
        Value::Typed { type_name, .. } => Some(type_name),
        _ => None,
    }
}

/// Whether `candidate` is `declared`, or a defined type whose alias chain
/// reaches it (`IfcPositiveLengthMeasure` is an `IfcLengthMeasure`).
pub(super) fn is_or_aliases(schema: &Schema, candidate: &str, declared: &str) -> bool {
    let mut current = candidate.trim().to_owned();
    for _ in 0..MAX_DEPTH {
        if current.eq_ignore_ascii_case(declared) {
            return true;
        }
        let Some(TypeKind::Defined(target)) = schema.type_def(&current).map(|d| &d.kind) else {
            return false;
        };
        current = target.trim().to_owned();
    }
    false
}

/// What a value is, in words.
pub(super) fn describe(value: &Value) -> String {
    match value {
        Value::Null => "$".to_owned(),
        Value::Derived => "*".to_owned(),
        Value::Bool(_) => "a bare boolean".to_owned(),
        Value::LogicalUnknown => "a bare .U.".to_owned(),
        Value::Integer(_) => "a bare integer".to_owned(),
        Value::Real(_) => "a bare real".to_owned(),
        Value::Text(_) => "a bare string".to_owned(),
        Value::Binary(_) => "a bare binary".to_owned(),
        Value::Enum(_) => "an enumeration constant".to_owned(),
        Value::Ref(_) => "an entity reference".to_owned(),
        Value::List(_) => "a list".to_owned(),
        Value::Typed { type_name, .. } => format!("a {type_name}"),
    }
}

/// The defined types the SELECT `name` closes over, upper-case: nested
/// SELECTs are walked, every other member is a leaf.
fn select_members(schema: &Schema, name: &str) -> BTreeSet<String> {
    let mut members = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut frontier = vec![name.to_owned()];
    while let Some(current) = frontier.pop() {
        if !seen.insert(current.to_ascii_uppercase()) {
            continue;
        }
        match schema.type_def(&current).map(|d| &d.kind) {
            Some(TypeKind::Select(list)) => frontier.extend(list.iter().cloned()),
            Some(_) if current != name => {
                members.insert(current.to_ascii_uppercase());
            }
            _ => {}
        }
    }
    members
}

/// Whether `payload` is a value of the declared type `type_text`: a type
/// name, a built-in, or an aggregate such as `ARRAY [1:2] OF REAL`.
fn fits(schema: &Schema, type_text: &str, payload: &Value, depth: usize) -> Result<(), String> {
    if depth == 0 {
        return Err("its declaration nests too deep".to_owned());
    }
    let resolved = schema.resolve_defined(type_text.trim());
    let resolved = resolved.trim();
    let head: String = resolved
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_ascii_uppercase();
    let ok = match head.as_str() {
        "REAL" => matches!(payload, Value::Real(_)),
        "INTEGER" => matches!(payload, Value::Integer(_)),
        "NUMBER" => matches!(payload, Value::Real(_) | Value::Integer(_)),
        "STRING" => matches!(payload, Value::Text(_)),
        "BOOLEAN" => matches!(payload, Value::Bool(_)),
        "LOGICAL" => matches!(payload, Value::Bool(_) | Value::LogicalUnknown),
        "BINARY" => matches!(payload, Value::Binary(_)),
        "LIST" | "ARRAY" | "SET" | "BAG" => return aggregate(schema, resolved, payload, depth),
        _ => {
            return Err(format!(
                "its type resolves to {resolved}, which no payload is checked against"
            ))
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!("expected {}", head.to_ascii_lowercase()))
    }
}

/// `payload` against an aggregate declaration: its bounds, then each member.
fn aggregate(schema: &Schema, declared: &str, payload: &Value, depth: usize) -> Result<(), String> {
    let Value::List(items) = payload else {
        return Err(format!("expected a list for {declared}"));
    };
    let (bounds, element) = declared
        .split_once(" OF ")
        .ok_or_else(|| format!("cannot read the aggregate {declared}"))?;
    if let Some((low, high)) = bounds
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .and_then(|(inside, _)| inside.split_once(':'))
    {
        let low: usize = low.trim().parse().unwrap_or(0);
        let high: Option<usize> = high.trim().parse().ok();
        if items.len() < low || high.is_some_and(|high| items.len() > high) {
            return Err(format!(
                "{} members where {declared} is declared",
                items.len()
            ));
        }
    }
    let element = element
        .trim()
        .trim_start_matches("UNIQUE ")
        .trim_start_matches("OPTIONAL ")
        .trim();
    items
        .iter()
        .try_for_each(|item| fits(schema, element, item, depth - 1))
}

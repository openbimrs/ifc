//! Which form an attribute's value is written in: typed parameter or bare.
//!
//! ISO 10303-21:2016 decides this from the declared type alone. A value of
//! a type that is not a SELECT is written bare (§12.1.6 for a simple defined
//! type, §12.1.7 for an enumeration): `IfcQuantityArea.AreaValue` holds
//! `12.5`, not `IFCAREAMEASURE(12.5)`. A value of a SELECT that is not an
//! entity instance is written as a typed parameter whose keyword names a type
//! in the select-list, nested SELECTs included (§12.1.8). A type that aliases
//! a SELECT is encoded as that SELECT (§12.1.8, EXAMPLE 2), so aliases are
//! followed before deciding.
//!
//! Like the rest of `check`, this answers only what the schema tables settle.
//! A declared type naming nothing the tables declare is [`Form::Unresolved`],
//! and the caller accepts any form there.

use ifc_schema::{Schema, TypeKind};

/// How many alias hops are followed. A malformed schema can declare
/// `TYPE A = B; TYPE B = A;`; this keeps that from hanging the writer.
const MAX_HOPS: usize = 16;

/// Upper bound on types visited in one select-list walk.
const MAX_VISITED: usize = 4096;

/// The form a declared type's values take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    /// A SELECT: typed, unless the value is an entity reference.
    Typed,
    /// Anything else the tables resolve: bare.
    Bare,
    /// The tables do not say; either form is accepted.
    Unresolved,
}

/// The form values of `type_name` take, following defined-type aliases.
pub(crate) fn form_of(schema: &Schema, type_name: &str) -> Form {
    let mut current = type_name.trim().to_owned();
    for _ in 0..MAX_HOPS {
        if schema.entity(&current).is_some() {
            return Form::Bare;
        }
        match schema.type_def(&current).map(|definition| &definition.kind) {
            Some(TypeKind::Select(_)) => return Form::Typed,
            Some(TypeKind::Enumeration(_)) => return Form::Bare,
            Some(TypeKind::Defined(target)) => current = target.trim().to_owned(),
            Some(_) => return Form::Unresolved,
            None if is_builtin(&current) => return Form::Bare,
            None => return Form::Unresolved,
        }
    }
    Form::Unresolved
}

/// Whether EXPRESS type text starts with a built-in simple or aggregate
/// type keyword: `STRING(255)`, `LIST [3:4] OF INTEGER`.
fn is_builtin(text: &str) -> bool {
    let head = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(
        head.as_str(),
        "REAL"
            | "INTEGER"
            | "NUMBER"
            | "STRING"
            | "BOOLEAN"
            | "LOGICAL"
            | "BINARY"
            | "LIST"
            | "SET"
            | "ARRAY"
            | "BAG"
    )
}

/// Whether `keyword` names a type in the select-list of the SELECT
/// `type_name` resolves to, or of a SELECT nested in it.
///
/// Only nested SELECTs are walked: a defined type in the list is a leaf, so
/// its underlying type is not a member. `None` when the answer would be "no"
/// but some member names nothing the tables declare, since that member
/// might have listed `keyword`, and when `type_name` is not a SELECT.
pub(crate) fn select_lists(schema: &Schema, type_name: &str, keyword: &str) -> Option<bool> {
    let mut start = type_name.trim().to_owned();
    for _ in 0..MAX_HOPS {
        match &schema.type_def(&start)?.kind {
            TypeKind::Select(_) => break,
            TypeKind::Defined(target) => start = target.trim().to_owned(),
            TypeKind::Enumeration(_) => return None,
            _ => return None,
        }
    }
    let mut frontier = vec![start];
    let mut seen = std::collections::BTreeSet::new();
    let mut unknown = false;
    while let Some(current) = frontier.pop() {
        if !seen.insert(current.to_ascii_uppercase()) {
            continue;
        }
        if seen.len() > MAX_VISITED {
            return None;
        }
        let Some(TypeKind::Select(members)) =
            schema.type_def(&current).map(|definition| &definition.kind)
        else {
            continue;
        };
        for member in members {
            if member.eq_ignore_ascii_case(keyword) {
                return Some(true);
            }
            if schema.type_def(member).is_none() && schema.entity(member).is_none() {
                unknown = true;
            }
            frontier.push(member.clone());
        }
    }
    (!unknown).then_some(false)
}

/// Whether `wrapper` is `declared`, or a defined type whose alias chain
/// reaches it (`IfcPositiveLengthMeasure` -> `IfcLengthMeasure`): a value of
/// the right type, whatever form it was written in.
pub(crate) fn aliases_to(schema: &Schema, wrapper: &str, declared: &str) -> bool {
    let mut current = wrapper.trim().to_owned();
    for _ in 0..MAX_HOPS {
        if current.eq_ignore_ascii_case(declared) {
            return true;
        }
        let Some(TypeKind::Defined(target)) =
            schema.type_def(&current).map(|definition| &definition.kind)
        else {
            return false;
        };
        current = target.trim().to_owned();
    }
    false
}

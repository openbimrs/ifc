//! Which type the members of an aggregate value are checked against.
//!
//! # Two ways a slot holds an aggregate
//!
//! An attribute can be declared as one (`Coordinates : LIST [1:3] OF
//! IfcLengthMeasure`), in which case the parser keeps the element type as
//! the slot's type token. Or its declared type can alias one: IFC4's
//! `IfcLineIndex = LIST [2:?] OF IfcPositiveInteger` is a scalar-declared
//! type whose values are lists. The parser keeps such a right-hand side as
//! text, so its element type is read back out of that text here.
//!
//! # Nested aggregates
//!
//! A nested attribute aggregate (`LIST OF LIST OF IfcLengthMeasure`) is
//! recorded with its innermost element type (#111), so the members of every
//! level are judged against `IfcLengthMeasure`; `structure::bounds` judges
//! the nesting itself. A defined type aliasing a nested aggregate still
//! yields its inner keyword from the alias text and leaves its members
//! unjudged.

use ifc_schema::{Schema, TypeKind};

/// How many alias hops are followed before giving up.
///
/// IFC alias chains are two or three long; this only stops a malformed
/// cyclic table from looping.
const MAX_ALIAS_HOPS: usize = 16;

/// The aggregate expression a named type aliases, if it aliases one.
///
/// `IfcComplexNumber` -> `ARRAY [1:2] OF REAL`; `IfcLabel` -> `None`.
#[must_use]
pub fn aliased_aggregate(schema: &Schema, type_name: &str) -> Option<String> {
    let mut current = type_name.to_string();
    for _ in 0..MAX_ALIAS_HOPS {
        let TypeKind::Defined(target) = &schema.type_def(&current)?.kind else {
            return None;
        };
        let target = target.trim();
        if is_aggregate_expression(target) {
            return Some(target.to_string());
        }
        current = target.to_string();
    }
    None
}

/// The type each member of a list written in a `declared` slot must have.
///
/// For a type aliasing an aggregate, its element type; otherwise `declared`
/// itself, which for an aggregate attribute is already the element token.
#[must_use]
pub fn element_type(schema: &Schema, declared: &str) -> String {
    aliased_aggregate(schema, declared)
        .and_then(|expression| element_of(&expression))
        .unwrap_or_else(|| declared.to_string())
}

/// Whether EXPRESS type text starts with an aggregate keyword.
fn is_aggregate_expression(text: &str) -> bool {
    let head = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(head.as_str(), "LIST" | "SET" | "ARRAY" | "BAG")
}

/// The element type named after the first `OF` of an aggregate expression.
///
/// `LIST [3:3] OF UNIQUE IfcPositiveInteger` -> `IfcPositiveInteger`. A
/// nested aggregate yields its own keyword (`LIST`), which names no type
/// and so leaves its members unjudged.
fn element_of(expression: &str) -> Option<String> {
    let mut tokens = expression.split_whitespace();
    tokens
        .by_ref()
        .find(|token| token.eq_ignore_ascii_case("OF"))?;
    let token = tokens.find(|token| {
        !token.eq_ignore_ascii_case("UNIQUE") && !token.eq_ignore_ascii_case("OPTIONAL")
    })?;
    let token = token.trim_matches(|c: char| matches!(c, '(' | ')' | ';'));
    (!token.is_empty()).then(|| token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_aliased_aggregate_yields_its_element_type() {
        let schema = ifc_schema::ifc4();
        assert_eq!(element_type(schema, "IfcComplexNumber"), "REAL");
        assert_eq!(element_type(schema, "IfcLineIndex"), "IfcPositiveInteger");
        assert_eq!(
            element_type(schema, "IfcPropertySetDefinitionSet"),
            "IfcPropertySetDefinition"
        );
    }

    #[test]
    fn a_non_aggregate_type_is_its_own_element_type() {
        let schema = ifc_schema::ifc4();
        assert_eq!(aliased_aggregate(schema, "IfcLengthMeasure"), None);
        assert_eq!(element_type(schema, "IfcLengthMeasure"), "IfcLengthMeasure");
        assert_eq!(element_type(schema, "IfcProduct"), "IfcProduct");
    }

    #[test]
    fn element_parsing_skips_qualifiers() {
        assert_eq!(
            element_of("LIST [3:?] OF UNIQUE IfcCartesianPoint").as_deref(),
            Some("IfcCartesianPoint")
        );
        assert_eq!(
            element_of("LIST [1:?] OF LIST [3:3] OF REAL").as_deref(),
            Some("LIST")
        );
        assert_eq!(element_of("STRING(255)"), None);
    }
}

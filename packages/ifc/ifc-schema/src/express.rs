//! Parser for the official EXPRESS (`.exp`) schema files.
//!
//! Reads a buildingSMART `SCHEMA ...; ... END_SCHEMA;` document into the
//! tables in [`crate::entity`] and [`crate::types`].
//!
//! # Scope, deliberately narrow
//!
//! This reads the *structure* needed to interpret files: entity names,
//! supertypes, explicit attribute slots, and type declarations. It does not
//! evaluate `WHERE` rules, `DERIVE` expressions, or `FUNCTION` bodies -- those
//! are a constraint language, and running them belongs to `ifc-validate`.
//!
//! # Why parse rather than vendor generated tables
//!
//! The `.exp` files are the normative artifacts and sit in `references/`. A
//! parser stays correct when a new addendum lands; hand-copied tables rot
//! silently and nobody notices until a file misreads.

use crate::attribute::Attribute;
use crate::entity::EntityDef;
use crate::types::{TypeDef, TypeKind};

/// Everything one `.exp` document declares.
#[derive(Debug, Default)]
pub struct ParsedSchema {
    /// Schema name from the `SCHEMA` header, e.g. `IFC4`.
    pub name: String,
    /// Entity declarations in file order.
    pub entities: Vec<EntityDef>,
    /// Type declarations in file order.
    pub types: Vec<TypeDef>,
}

/// Parse an EXPRESS schema document.
///
/// Never fails: an unrecognized construct is skipped rather than rejected, on
/// the same principle as the STEP reader. A schema this parser only partly
/// understands is still more useful than an error.
pub fn parse(source: &str) -> ParsedSchema {
    let mut out = ParsedSchema::default();
    let text = strip_comments(source);
    let mut rest = text.as_str();

    if let Some(i) = rest.find("SCHEMA ") {
        let after = &rest[i + 7..];
        if let Some(end) = after.find(';') {
            out.name = after[..end].trim().to_string();
        }
    }

    while let Some(pos) = next_decl(rest) {
        let (kind, start) = pos;
        let body_start = start + kind.len();
        let terminator = match kind {
            "ENTITY " => "END_ENTITY;",
            _ => "END_TYPE;",
        };
        let Some(end) = rest[body_start..].find(terminator) else {
            break;
        };
        let body = &rest[body_start..body_start + end];

        match kind {
            "ENTITY " => {
                if let Some(e) = parse_entity(body) {
                    out.entities.push(e);
                }
            }
            _ => {
                if let Some(t) = parse_type(body) {
                    out.types.push(t);
                }
            }
        }
        rest = &rest[body_start + end + terminator.len()..];
    }
    out
}

/// Find the next `ENTITY ` or `TYPE ` declaration at a statement boundary.
///
/// Matching bare substrings would fire on `ENTITY` inside a comment or on the
/// `TYPE` in `PREDEFINEDTYPE`, so a boundary check is required.
fn next_decl(text: &str) -> Option<(&'static str, usize)> {
    let e = find_at_boundary(text, "ENTITY ");
    let t = find_at_boundary(text, "TYPE ");
    match (e, t) {
        (Some(a), Some(b)) if a <= b => Some(("ENTITY ", a)),
        (Some(_), Some(b)) => Some(("TYPE ", b)),
        (Some(a), None) => Some(("ENTITY ", a)),
        (None, Some(b)) => Some(("TYPE ", b)),
        (None, None) => None,
    }
}

/// Find `needle` only where it starts a statement.
fn find_at_boundary(text: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = text[from..].find(needle) {
        let abs = from + i;
        let preceded_ok = abs == 0
            || text[..abs]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == ';');
        // `END_TYPE;` and `END_ENTITY;` must not be mistaken for openers.
        let is_end = abs >= 4 && text[..abs].ends_with("END_");
        if preceded_ok && !is_end {
            return Some(abs);
        }
        from = abs + needle.len();
    }
    None
}

/// Strip `(* ... *)` comments so keywords inside prose cannot be parsed.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            match source[i + 2..].find("*)") {
                Some(end) => i += 2 + end + 2,
                None => break,
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Parse one `ENTITY` body (between the keyword and `END_ENTITY;`).
fn parse_entity(body: &str) -> Option<EntityDef> {
    let semi = body.find(';')?;
    let header = &body[..semi];
    let rest = &body[semi + 1..];

    let mut header_words = header.split_whitespace();
    let name = header_words.next()?.trim_end_matches(';').to_string();

    let upper = header.to_ascii_uppercase();
    let mut def = EntityDef::new(&name);
    def.abstract_ = upper.contains("ABSTRACT");

    if let Some(i) = upper.find("SUBTYPE OF") {
        let after = &header[i + "SUBTYPE OF".len()..];
        if let Some(open) = after.find('(') {
            if let Some(close) = after[open..].find(')') {
                let sup = after[open + 1..open + close].trim();
                if !sup.is_empty() {
                    def.supertype = Some(sup.to_string());
                }
            }
        }
    }

    // Attributes run until the first section keyword. Everything after
    // DERIVE/INVERSE/WHERE/UNIQUE is not a positional slot.
    let attr_text = split_before_sections(rest);
    for stmt in attr_text.split(';') {
        if let Some(attr) = parse_attribute(stmt) {
            def.attributes.push(attr);
        }
    }
    Some(def)
}

/// Truncate at the first `DERIVE`/`INVERSE`/`WHERE`/`UNIQUE` section keyword.
fn split_before_sections(body: &str) -> &str {
    let upper = body.to_ascii_uppercase();
    let mut cut = body.len();
    for kw in ["DERIVE", "INVERSE", "WHERE", "UNIQUE"] {
        if let Some(i) = find_at_boundary(&upper, kw) {
            cut = cut.min(i);
        }
    }
    &body[..cut]
}

/// Parse one `Name : TYPE;` attribute statement.
fn parse_attribute(stmt: &str) -> Option<Attribute> {
    let stmt = stmt.trim();
    if stmt.is_empty() {
        return None;
    }
    let colon = stmt.find(':')?;
    let name = stmt[..colon].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let decl = stmt[colon + 1..].trim();
    let upper = decl.to_ascii_uppercase();

    let optional = upper.starts_with("OPTIONAL");
    let decl = if optional {
        decl["OPTIONAL".len()..].trim()
    } else {
        decl
    };
    let upper = decl.to_ascii_uppercase();
    let aggregate = ["LIST", "SET", "ARRAY", "BAG"]
        .iter()
        .any(|k| upper.starts_with(k));

    // The base type is the last token: `LIST [1:?] OF IfcCartesianPoint`.
    let type_name = decl
        .split_whitespace()
        .next_back()
        .unwrap_or(decl)
        .trim_end_matches(';')
        .to_string();

    let mut attr = Attribute::new(name, type_name);
    attr.optional = optional;
    attr.aggregate = aggregate;
    Some(attr)
}

/// Parse one `TYPE` body (between the keyword and `END_TYPE;`).
fn parse_type(body: &str) -> Option<TypeDef> {
    let eq = body.find('=')?;
    let name = body[..eq].trim().to_string();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let decl = body[eq + 1..].trim();
    let upper = decl.to_ascii_uppercase();

    let kind = if upper.starts_with("ENUMERATION") {
        TypeKind::Enumeration(collect_parenthesized(decl))
    } else if upper.starts_with("SELECT") {
        TypeKind::Select(collect_parenthesized(decl))
    } else {
        let base = decl.split([';', ' ']).next().unwrap_or(decl).trim();
        TypeKind::Defined(base.to_string())
    };
    Some(TypeDef { name, kind })
}

/// Collect comma-separated names inside the first `( ... )`.
fn collect_parenthesized(decl: &str) -> Vec<String> {
    let Some(open) = decl.find('(') else {
        return Vec::new();
    };
    let Some(close) = decl[open..].rfind(')') else {
        return Vec::new();
    };
    decl[open + 1..open + close]
        .split(',')
        .map(|s| s.trim().trim_end_matches(';').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
SCHEMA IFC4;

TYPE IfcLengthMeasure = REAL;
END_TYPE;

TYPE IfcWallTypeEnum = ENUMERATION OF
  (MOVABLE, PARAPET, PARTITIONING, USERDEFINED, NOTDEFINED);
END_TYPE;

TYPE IfcValue = SELECT
  (IfcMeasureValue, IfcSimpleValue);
END_TYPE;

ENTITY IfcRoot
 ABSTRACT SUPERTYPE OF (ONEOF(IfcObjectDefinition));
  GlobalId : IfcGloballyUniqueId;
  OwnerHistory : OPTIONAL IfcOwnerHistory;
  Name : OPTIONAL IfcLabel;
END_ENTITY;

ENTITY IfcCartesianPoint
  SUBTYPE OF (IfcPoint);
  Coordinates : LIST [1:3] OF IfcLengthMeasure;
 DERIVE
  Dim : IfcDimensionCount := HIINDEX(Coordinates);
 WHERE
  CP2Dor3D : HIINDEX(Coordinates) >= 2;
END_ENTITY;

END_SCHEMA;
";

    #[test]
    fn reads_schema_name() {
        assert_eq!(parse(SAMPLE).name, "IFC4");
    }

    #[test]
    fn reads_entities_with_supertype_and_abstractness() {
        let s = parse(SAMPLE);
        let root = s.entities.iter().find(|e| e.name == "IfcRoot").unwrap();
        assert!(root.abstract_, "IfcRoot is ABSTRACT");
        assert_eq!(root.supertype, None);

        let cp = s
            .entities
            .iter()
            .find(|e| e.name == "IfcCartesianPoint")
            .unwrap();
        assert_eq!(cp.supertype.as_deref(), Some("IfcPoint"));
        assert!(!cp.abstract_);
    }

    /// The bug this guards: including DERIVE or INVERSE members would shift
    /// every later attribute index and silently misread files.
    #[test]
    fn derive_and_where_members_are_not_attribute_slots() {
        let s = parse(SAMPLE);
        let cp = s
            .entities
            .iter()
            .find(|e| e.name == "IfcCartesianPoint")
            .unwrap();
        assert_eq!(cp.attributes.len(), 1, "only Coordinates is a slot");
        assert_eq!(cp.attributes[0].name, "Coordinates");
        assert!(cp.attributes[0].aggregate, "LIST is an aggregate");
    }

    #[test]
    fn reads_optionality() {
        let s = parse(SAMPLE);
        let root = s.entities.iter().find(|e| e.name == "IfcRoot").unwrap();
        assert_eq!(root.attributes.len(), 3);
        assert!(!root.attributes[0].optional, "GlobalId is required");
        assert!(root.attributes[1].optional, "OwnerHistory is OPTIONAL");
        assert_eq!(root.attributes[1].type_name, "IfcOwnerHistory");
    }

    #[test]
    fn reads_the_three_type_kinds() {
        let s = parse(SAMPLE);
        assert_eq!(s.types.len(), 3);
        match &s.types[0].kind {
            TypeKind::Defined(base) => assert_eq!(base, "REAL"),
            other => panic!("expected defined type, got {other:?}"),
        }
        match &s.types[1].kind {
            TypeKind::Enumeration(v) => {
                assert_eq!(v.len(), 5);
                assert_eq!(v[0], "MOVABLE");
            }
            other => panic!("expected enumeration, got {other:?}"),
        }
        match &s.types[2].kind {
            TypeKind::Select(v) => assert_eq!(v.len(), 2),
            other => panic!("expected select, got {other:?}"),
        }
    }

    #[test]
    fn comments_do_not_produce_declarations() {
        let src = "SCHEMA X; (* ENTITY NotReal; a : b; END_ENTITY; *) END_SCHEMA;";
        assert!(parse(src).entities.is_empty());
    }
}

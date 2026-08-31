//! Does a written value match the primitive its declared type resolves to?
//!
//! # The defect this exists to catch
//!
//! ```text
//! #1=IFCPROPERTYSINGLEVALUE('x',$,IFCPOSITIVELENGTHMEASURE('1'),$);
//!                                                          ^^^
//! ```
//!
//! `IfcPositiveLengthMeasure` resolves to `REAL`, and `'1'` is a string. The
//! file parses -- Part 21 has no idea what the token means -- and every
//! consumer that reads a number from that slot gets nothing. This is a real
//! IfcOpenShell regression fixture, not a hypothetical.

use ifc_model::Value;
use ifc_schema::Schema;

/// A `STRING(n) FIXED` width declared by a type.
///
/// EXPRESS lets a string type fix its own width: `IfcGloballyUniqueId` is
/// `STRING(22) FIXED`, and a 21-character GUID is malformed regardless of
/// whether it is unique. The width is part of the type expression the parser
/// keeps as text, so it is read back out here rather than special-cased per
/// type -- any `FIXED` width in any schema is checked the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedWidth(pub usize);

impl FixedWidth {
    /// Reads a `STRING(n) FIXED` width from a resolved type expression.
    ///
    /// Returns `None` for a plain `STRING`, a bounded-but-not-fixed
    /// `STRING(n)`, or anything that is not a string: only `FIXED` makes the
    /// width a conformance requirement rather than a maximum.
    #[must_use]
    pub fn from_resolved(resolved: &str) -> Option<Self> {
        let upper = resolved.to_ascii_uppercase();
        if !upper.contains("FIXED") {
            return None;
        }
        let start = upper.find("STRING")?;
        let open = upper[start..].find('(')? + start;
        let close = upper[open..].find(')')? + open;
        upper[open + 1..close].trim().parse().ok().map(Self)
    }

    /// Whether `text` has exactly the declared width.
    ///
    /// Counted in characters, not bytes: a Latin-1 accented character is one
    /// character in EXPRESS terms.
    #[must_use]
    pub fn accepts(self, text: &str) -> bool {
        text.chars().count() == self.0
    }
}

/// The EXPRESS primitive a defined type bottoms out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    /// `REAL` or `NUMBER`.
    Real,
    /// `INTEGER`.
    Integer,
    /// `STRING`.
    Text,
    /// `BOOLEAN`.
    Boolean,
    /// `LOGICAL` -- true, false, *or* unknown.
    Logical,
    /// `BINARY`.
    Binary,
}

impl Primitive {
    /// Recognizes an EXPRESS primitive from a resolved type expression.
    ///
    /// The resolved text may carry aggregate syntax (`LIST [1:?] OF REAL`);
    /// only the trailing primitive token is examined, and anything
    /// unrecognized yields `None` rather than a guess.
    #[must_use]
    pub fn from_resolved(resolved: &str) -> Option<Self> {
        let token = resolved
            .rsplit(|c: char| c.is_whitespace() || c == '(')
            .find(|part| !part.is_empty())?
            .trim_end_matches([')', ';'])
            .to_ascii_uppercase();
        match token.as_str() {
            "REAL" | "NUMBER" => Some(Self::Real),
            "INTEGER" => Some(Self::Integer),
            "STRING" => Some(Self::Text),
            "BOOLEAN" => Some(Self::Boolean),
            "LOGICAL" => Some(Self::Logical),
            "BINARY" => Some(Self::Binary),
            _ => None,
        }
    }

    /// Whether `value` is written in a form this primitive accepts.
    ///
    /// An `INTEGER` is accepted where a `REAL` is declared: Part 21 writes
    /// `1.` and `1` for the same quantity depending on the writer, and
    /// rejecting the integer spelling would fail almost every real file.
    /// The reverse is not accepted -- `1.5` is not an `INTEGER`.
    #[must_use]
    pub fn accepts(self, value: &Value) -> bool {
        match self {
            Self::Real => matches!(value, Value::Real(_) | Value::Integer(_)),
            Self::Integer => matches!(value, Value::Integer(_)),
            Self::Text => matches!(value, Value::Text(_)),
            Self::Boolean => matches!(value, Value::Bool(_)),
            Self::Logical => matches!(value, Value::Bool(_) | Value::LogicalUnknown),
            Self::Binary => matches!(value, Value::Binary(_)),
        }
    }

    /// How this primitive is named in a diagnostic.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Real => "a number",
            Self::Integer => "an integer",
            Self::Text => "a string",
            Self::Boolean => "a boolean",
            Self::Logical => "a logical",
            Self::Binary => "binary data",
        }
    }
}

/// How a value was actually written, for a diagnostic.
#[must_use]
pub fn describe_value(value: &Value) -> &'static str {
    match value {
        Value::Null => "unset",
        Value::Derived => "derived",
        Value::Bool(_) => "a boolean",
        Value::LogicalUnknown => "unknown",
        Value::Integer(_) => "an integer",
        Value::Real(_) => "a number",
        Value::Text(_) => "a string",
        Value::Binary(_) => "binary data",
        Value::Enum(_) => "an enumeration constant",
        Value::Ref(_) => "a reference",
        Value::List(_) => "an aggregate",
        Value::Typed { .. } => "a typed value",
    }
}

/// The primitive a declared type resolves to, if any.
#[must_use]
pub fn primitive_of(schema: &Schema, type_name: &str) -> Option<Primitive> {
    Primitive::from_resolved(&schema.resolve_defined(type_name))
}

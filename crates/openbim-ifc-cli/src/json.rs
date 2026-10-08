//! A small JSON writer for the machine-readable outputs.
//!
//! The documents are built as [`Json`] values and written once, with object
//! keys in the order they were inserted so the output reads in the order
//! the docs describe it. Only writing is needed; a JSON crate would bring a
//! parser and a derive layer for no gain.

use std::fmt::{self, Write as _};

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A finite number; a non-finite one is written as `null`.
    Real(f64),
    /// A string.
    Str(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, keys in insertion order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// An object from `(key, value)` pairs.
    pub(crate) fn object<K: Into<String>>(fields: impl IntoIterator<Item = (K, Json)>) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    /// A string.
    pub(crate) fn str(text: impl Into<String>) -> Self {
        Self::Str(text.into())
    }

    /// A string, or `null`.
    pub(crate) fn opt_str(text: Option<impl Into<String>>) -> Self {
        text.map_or(Self::Null, |text| Self::Str(text.into()))
    }

    /// An unsigned count or id; ids past `i64::MAX` do not occur in a
    /// STEP file, whose ids are written in decimal, and saturate.
    pub(crate) fn uint(value: u64) -> Self {
        Self::Int(i64::try_from(value).unwrap_or(i64::MAX))
    }

    /// An unsigned id or count, or `null`.
    pub(crate) fn opt_uint(value: Option<u64>) -> Self {
        value.map_or(Self::Null, Self::uint)
    }

    /// The value written with two-space indentation and a final newline.
    pub(crate) fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0).expect("writing to a String");
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, depth: usize) -> fmt::Result {
        match self {
            Self::Null => out.write_str("null"),
            Self::Bool(value) => write!(out, "{value}"),
            Self::Int(value) => write!(out, "{value}"),
            Self::Real(value) if value.is_finite() => {
                // `{:?}` keeps a decimal point (`2.0`) and round-trips.
                write!(out, "{value:?}")
            }
            Self::Real(_) => out.write_str("null"),
            Self::Str(text) => write_string(out, text),
            Self::Array(items) if items.is_empty() => out.write_str("[]"),
            Self::Array(items) => {
                out.write_str("[\n")?;
                for (index, item) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    item.write(out, depth + 1)?;
                    out.write_str(if index + 1 < items.len() { ",\n" } else { "\n" })?;
                }
                indent(out, depth);
                out.write_str("]")
            }
            Self::Object(fields) if fields.is_empty() => out.write_str("{}"),
            Self::Object(fields) => {
                out.write_str("{\n")?;
                for (index, (key, value)) in fields.iter().enumerate() {
                    indent(out, depth + 1);
                    write_string(out, key)?;
                    out.write_str(": ")?;
                    value.write(out, depth + 1)?;
                    out.write_str(if index + 1 < fields.len() {
                        ",\n"
                    } else {
                        "\n"
                    })?;
                }
                indent(out, depth);
                out.write_str("}")
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/// A JSON string literal (RFC 8259 section 7).
fn write_string(out: &mut String, text: &str) -> fmt::Result {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => write!(out, "\\u{:04x}", u32::from(c))?,
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_and_numbers_are_escaped_and_exact() {
        let value = Json::object([
            ("text", Json::str("a\"b\\c\n\u{1}é")),
            ("real", Json::Real(2.0)),
            ("nan", Json::Real(f64::NAN)),
            ("list", Json::Array(vec![Json::Int(1), Json::Null])),
            ("empty", Json::Array(Vec::new())),
        ]);
        assert_eq!(
            value.pretty(),
            "{\n  \"text\": \"a\\\"b\\\\c\\n\\u0001é\",\n  \"real\": 2.0,\n  \"nan\": null,\n  \
             \"list\": [\n    1,\n    null\n  ],\n  \"empty\": []\n}\n"
        );
    }
}

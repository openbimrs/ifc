//! [`Model`] to ifcXML text.
//!
//! # The lossless-encoding problem
//!
//! STEP distinguishes `$` (unset), `*` (derived), `.T.` (logical true),
//! `.ELEMENT.` (enum), `'text'` (string) and `1.` (real) syntactically. XML
//! attribute values are all just strings, so a naive writer collapses those
//! distinctions and the round-trip loses type information.
//!
//! This writer preserves the distinction structurally: scalars become
//! attributes, and anything whose *kind* cannot be inferred from an attribute
//! string is written as a typed child element. That keeps the output readable
//! for the common case while remaining exactly reversible.

use crate::error::XmlError;
use crate::XmlCodec;
use ifc_model::{Model, Value};
use std::fmt::Write as _;

/// Serialize a model as ifcXML.
pub fn write(codec: &XmlCodec, model: &Model) -> Result<Vec<u8>, XmlError> {
    let mut out = String::with_capacity(model.len() * 96);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<ifcXML xmlns=\"http://www.buildingsmart-tech.org/ifcXML/IFC4/final\"");
    let schema_token = model.header().schema.first().cloned().unwrap_or_default();
    write_attr(&mut out, "schema", &schema_token);
    out.push_str(">\n");

    write_header(&mut out, model);

    for (id, entity) in model.iter() {
        let names = attribute_names(codec, &entity.type_name, entity.attributes.len());

        write!(out, "  <{}", entity.type_name).map_err(fmt_err)?;
        write!(out, " id=\"i{}\"", id.0).map_err(fmt_err)?;

        // Scalars become XML attributes; everything else becomes a child.
        let mut children: Vec<(usize, &Value)> = Vec::new();
        for (i, value) in entity.attributes.iter().enumerate() {
            match scalar_text(value) {
                Some(text) => write_attr(&mut out, &names[i], &text),
                None => children.push((i, value)),
            }
        }

        if children.is_empty() {
            out.push_str("/>\n");
        } else {
            out.push_str(">\n");
            for (i, value) in children {
                write_child(&mut out, &names[i], value, 2)?;
            }
            writeln!(out, "  </{}>", entity.type_name).map_err(fmt_err)?;
        }
    }

    out.push_str("</ifcXML>\n");
    Ok(out.into_bytes())
}

fn fmt_err(e: std::fmt::Error) -> XmlError {
    XmlError::Write(e.to_string())
}

/// Format a real so it survives a round-trip.
///
/// XML has no real/integer distinction, so `1.0` must not be written as `1` --
/// re-reading would infer an integer and change the value's kind. A decimal
/// point is always present.
fn format_real(r: f64) -> String {
    if r == r.trunc() && r.is_finite() && r.abs() < 1e15 {
        format!("{r:.1}")
    } else {
        let s = format!("{r}");
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{s}.0")
        }
    }
}

/// Attribute names for a type: from the schema when available, else `a<i>`.
///
/// The fallback is deliberately obvious rather than a plausible guess: a wrong
/// name that looks right is worse than one that is visibly a placeholder.
fn attribute_names(codec: &XmlCodec, type_name: &str, count: usize) -> Vec<String> {
    #[cfg(feature = "schema")]
    if let Some(schema) = codec.schema() {
        let names = schema.attribute_names(type_name);
        if names.len() >= count {
            return names.iter().take(count).map(|s| s.to_string()).collect();
        }
    }
    let _ = (codec, type_name);
    (0..count).map(|i| format!("a{i}")).collect()
}

/// The header, mirroring STEP's `FILE_DESCRIPTION`/`FILE_NAME` content.
fn write_header(out: &mut String, model: &Model) {
    let h = model.header();
    out.push_str("  <header>\n");
    push_element(out, "name", &h.name);
    push_element(out, "time_stamp", &h.time_stamp);
    push_element(out, "preprocessor_version", &h.preprocessor_version);
    push_element(out, "originating_system", &h.originating_system);
    push_element(out, "authorization", &h.authorization);
    for a in &h.author {
        push_element(out, "author", a);
    }
    for o in &h.organization {
        push_element(out, "organization", o);
    }
    for d in &h.description {
        push_element(out, "description", d);
    }
    out.push_str("  </header>\n");
}

fn push_element(out: &mut String, tag: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    out.push_str("    <");
    out.push_str(tag);
    out.push('>');
    escape_into(out, text);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// A value representable as a plain XML attribute, or `None` if it needs an
/// element to stay reversible.
fn scalar_text(value: &Value) -> Option<String> {
    match value {
        // A string that LOOKS numeric ("0.1" as a version label) must not be
        // re-read as a number. Such strings go to a typed child element
        // instead; only unambiguous text stays an attribute.
        Value::Text(s) => (!looks_numeric(s)).then(|| s.to_string()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Real(r) => Some(format_real(*r)),
        Value::Ref(id) => Some(format!("i{}", id.0)),
        // Kinds below are NOT attribute-representable without ambiguity.
        Value::Null | Value::Derived | Value::Enum(_) => None,
        Value::Bool(_) | Value::LogicalUnknown | Value::Binary(_) => None,
        Value::List(_) | Value::Typed { .. } => None,
    }
}

/// Would this text be inferred as a number or reference when re-read?
///
/// Must mirror `reader::infer_scalar` exactly: any string it would promote to
/// a non-string kind has to be written as an element instead. The two
/// functions are a matched pair, which is why the round-trip test asserts on
/// value *kinds* rather than only on text.
fn looks_numeric(text: &str) -> bool {
    if text
        .strip_prefix('i')
        .is_some_and(|r| r.parse::<u64>().is_ok())
    {
        return true;
    }
    text.parse::<i64>().is_ok() || text.parse::<f64>().is_ok()
}

/// Write a value that needs its own element to preserve its kind.
fn write_child(out: &mut String, name: &str, value: &Value, depth: usize) -> Result<(), XmlError> {
    let pad = "  ".repeat(depth);
    match value {
        Value::Null => {
            writeln!(out, "{pad}<{name} xsi:nil=\"true\"/>").map_err(fmt_err)?;
        }
        Value::Derived => {
            writeln!(out, "{pad}<{name} derived=\"true\"/>").map_err(fmt_err)?;
        }
        Value::Enum(e) => {
            write!(out, "{pad}<{name} kind=\"enum\">").map_err(fmt_err)?;
            escape_into(out, e);
            writeln!(out, "</{name}>").map_err(fmt_err)?;
        }
        Value::Binary(b) => {
            write!(out, "{pad}<{name} kind=\"binary\">").map_err(fmt_err)?;
            escape_into(out, b);
            writeln!(out, "</{name}>").map_err(fmt_err)?;
        }
        Value::Bool(b) => {
            let text = if *b { "true" } else { "false" };
            writeln!(out, "{pad}<{name} kind=\"logical\">{text}</{name}>").map_err(fmt_err)?;
        }
        Value::LogicalUnknown => {
            writeln!(out, "{pad}<{name} kind=\"logical\">unknown</{name}>").map_err(fmt_err)?;
        }
        Value::Typed { type_name, value } => {
            writeln!(out, "{pad}<{name} kind=\"typed\" type=\"{type_name}\">").map_err(fmt_err)?;
            write_child(out, "value", value, depth + 1)?;
            writeln!(out, "{pad}</{name}>").map_err(fmt_err)?;
        }
        Value::List(items) => {
            writeln!(out, "{pad}<{name} kind=\"list\">").map_err(fmt_err)?;
            for item in items {
                write_child(out, "item", item, depth + 1)?;
            }
            writeln!(out, "{pad}</{name}>").map_err(fmt_err)?;
        }
        scalar => {
            // Must NOT go through `scalar_text`: that function returns None
            // for exactly the strings routed here, which would write an empty
            // element and silently lose the value.
            let (kind, text) = match scalar {
                Value::Text(s) => ("string", s.to_string()),
                Value::Integer(i) => ("integer", i.to_string()),
                Value::Real(r) => ("real", format_real(*r)),
                Value::Ref(id) => ("ref", format!("i{}", id.0)),
                other => ("string", scalar_text(other).unwrap_or_default()),
            };
            write!(out, "{pad}<{name} kind=\"{kind}\">").map_err(fmt_err)?;
            escape_into(out, &text);
            writeln!(out, "</{name}>").map_err(fmt_err)?;
        }
    }
    Ok(())
}

fn write_attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape_into(out, value);
    out.push('"');
}

/// XML-escape into an existing buffer.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
}

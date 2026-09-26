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
use crate::scalar::{attribute_text, element_form, format_ref};
use crate::{slots, XmlCodec};
use ifc_model::{Model, Value};
use std::fmt::Write as _;

const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

fn reject_non_finite_reals(model: &Model) -> Result<(), XmlError> {
    for (id, entity) in model.iter() {
        for (slot, value) in entity.attributes.iter().enumerate() {
            if let Some(non_finite) = first_non_finite_real(value) {
                let path = format!("/ifcXML/{}[@id='i{}']/a{}", entity.type_name, id.0, slot);
                return Err(XmlError::InvalidScalar {
                    kind: "real".into(),
                    value: non_finite.to_string(),
                }
                .at(path));
            }
        }
    }
    Ok(())
}

fn first_non_finite_real(value: &Value) -> Option<f64> {
    match value {
        Value::Real(value) if !value.is_finite() => Some(*value),
        Value::List(values) => values.iter().find_map(first_non_finite_real),
        Value::Typed { value, .. } => first_non_finite_real(value),
        _ => None,
    }
}

/// Serialize a model as ifcXML.
pub fn write(codec: &XmlCodec, model: &Model) -> Result<Vec<u8>, XmlError> {
    reject_non_finite_reals(model)?;
    let mut out = String::with_capacity(model.len() * 96);
    let schema_token = model.header().schema.first().cloned().unwrap_or_default();
    let namespace = if let Some(profile) = codec.profile() {
        if schema_token != profile.schema_token() {
            return Err(XmlError::Profile {
                expected: profile.schema_token(),
                found: (!schema_token.is_empty()).then_some(schema_token),
            });
        }
        profile.namespace()
    } else {
        "http://www.buildingsmart-tech.org/ifcXML/IFC4/final"
    };

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write!(
        out,
        "<ifcXML xmlns=\"{namespace}\" xmlns:xsi=\"{XSI_NAMESPACE}\""
    )
    .map_err(fmt_err)?;
    write_attr(&mut out, "schema", &schema_token);
    out.push_str(">\n");

    write_header(&mut out, model);

    for (id, entity) in model.iter() {
        let names = slots::attribute_names(codec, &entity.type_name, entity.attributes.len());

        write!(out, "  <{}", entity.type_name).map_err(fmt_err)?;
        write_attr(&mut out, "id", &format_ref(id));

        // Scalars become XML attributes; everything else becomes a child.
        let mut children: Vec<(usize, &Value)> = Vec::new();
        for (i, value) in entity.attributes.iter().enumerate() {
            match attribute_text(value) {
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
        Value::Typed { type_name, value } => {
            write!(out, "{pad}<{name} kind=\"typed\"").map_err(fmt_err)?;
            write_attr(out, "type", type_name);
            out.push_str(">\n");
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
        leaf => {
            // Leaf kinds always carry an explicit `kind` here, including the
            // text, numbers and references that `attribute_text` refused: an
            // element without one would be re-read by inference.
            let (kind, text) = element_form(leaf)
                .ok_or_else(|| XmlError::Write(format!("no element form for value {leaf:?}")))?;
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
///
/// Tab, line feed and carriage return are written as character references:
/// a conforming parser normalizes them to spaces inside attribute values and
/// folds CR/LF line ends in text, so a literal one would not survive.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
}

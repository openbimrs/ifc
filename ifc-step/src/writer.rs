//! [`Model`] to STEP text — the serialization stage.
//!
//! # Fidelity claim, stated precisely
//!
//! Output is **semantically** identical to the input, not byte-identical.
//! Real-number lexemes normalize (`1.` and `1.0` both write as `1.`), and
//! comments are not preserved. What *is* guaranteed:
//!
//! - every entity, with its original `#id`;
//! - original file order;
//! - every attribute value, including ones whose meaning this build has no
//!   crate for;
//! - text content through the escape codec.
//!
//! `tests/roundtrip.rs` verifies this by re-parsing the output and comparing
//! the two models structurally, which is the honest form of the claim.

use crate::escape;
use ifc_model::header::Header;
use ifc_model::{Model, Value};
use std::io::Write;

/// Write `model` as a STEP physical file.
pub fn write(model: &Model, out: &mut dyn Write) -> std::io::Result<()> {
    writeln!(out, "ISO-10303-21;")?;
    writeln!(out, "HEADER;")?;
    write_header(model.header(), out)?;
    writeln!(out, "ENDSEC;")?;
    writeln!(out, "DATA;")?;
    for (id, entity) in model.iter() {
        write!(out, "#{}= {}(", id.0, entity.type_name)?;
        write_values(&entity.attributes, out)?;
        writeln!(out, ");")?;
    }
    writeln!(out, "ENDSEC;")?;
    writeln!(out, "END-ISO-10303-21;")?;
    Ok(())
}

fn write_header(header: &Header, out: &mut dyn Write) -> std::io::Result<()> {
    write!(out, "FILE_DESCRIPTION(")?;
    write_string_list(&header.description, out)?;
    writeln!(out, ",'{}');", escape::encode(&header.implementation_level))?;

    write!(out, "FILE_NAME('{}'", escape::encode(&header.name))?;
    write!(out, ",'{}',", escape::encode(&header.time_stamp))?;
    write_string_list(&header.author, out)?;
    write!(out, ",")?;
    write_string_list(&header.organization, out)?;
    writeln!(
        out,
        ",'{}','{}','{}');",
        escape::encode(&header.preprocessor_version),
        escape::encode(&header.originating_system),
        escape::encode(&header.authorization)
    )?;

    write!(out, "FILE_SCHEMA(")?;
    write_string_list(&header.schema, out)?;
    writeln!(out, ");")?;
    Ok(())
}

fn write_string_list(items: &[String], out: &mut dyn Write) -> std::io::Result<()> {
    write!(out, "(")?;
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            write!(out, ",")?;
        }
        write!(out, "'{}'", escape::encode(item))?;
    }
    write!(out, ")")
}

fn write_values(values: &[Value], out: &mut dyn Write) -> std::io::Result<()> {
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            write!(out, ",")?;
        }
        write_value(value, out)?;
    }
    Ok(())
}

/// Write one value in STEP syntax.
pub fn write_value(value: &Value, out: &mut dyn Write) -> std::io::Result<()> {
    match value {
        Value::Null => write!(out, "$"),
        Value::Derived => write!(out, "*"),
        Value::Bool(true) => write!(out, ".T."),
        Value::Bool(false) => write!(out, ".F."),
        Value::LogicalUnknown => write!(out, ".U."),
        Value::Integer(i) => write!(out, "{i}"),
        Value::Real(r) => write!(out, "{}", format_real(*r)),
        Value::Text(s) => write!(out, "'{}'", escape::encode(s)),
        Value::Binary(b) => write!(out, "\"{b}\""),
        Value::Enum(e) => write!(out, ".{e}."),
        Value::Ref(id) => write!(out, "#{}", id.0),
        Value::List(items) => {
            write!(out, "(")?;
            write_values(items, out)?;
            write!(out, ")")
        }
        Value::Typed { type_name, value } => {
            write!(out, "{type_name}(")?;
            write_value(value, out)?;
            write!(out, ")")
        }
    }
}

/// Format a real the way STEP expects.
///
/// A STEP real must carry a decimal point or an exponent, so `2` would be
/// invalid where `2.` is required; readers that accept the former are being
/// lenient, and relying on that leniency breaks interoperability.
fn format_real(r: f64) -> String {
    if r == r.trunc() && r.abs() < 1e15 {
        format!("{}.", r.trunc() as i64)
    } else {
        let s = format!("{r}");
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{s}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::{Entity, EntityId};

    fn render(value: &Value) -> String {
        let mut buf = Vec::new();
        write_value(value, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn writes_step_real_syntax() {
        assert_eq!(render(&Value::Real(2.0)), "2.");
        assert_eq!(render(&Value::Real(0.2)), "0.2");
        assert_eq!(render(&Value::Real(-1.5)), "-1.5");
    }

    #[test]
    fn writes_the_three_logical_states_distinctly() {
        assert_eq!(render(&Value::Bool(true)), ".T.");
        assert_eq!(render(&Value::Bool(false)), ".F.");
        assert_eq!(render(&Value::LogicalUnknown), ".U.");
    }

    #[test]
    fn writes_null_and_derived_distinctly() {
        assert_eq!(render(&Value::Null), "$");
        assert_eq!(render(&Value::Derived), "*");
    }

    #[test]
    fn writes_nested_typed_values() {
        let v = Value::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Value::Real(0.2)),
        };
        assert_eq!(render(&v), "IFCLENGTHMEASURE(0.2)");
    }

    #[test]
    fn writes_entities_in_file_order_with_original_ids() {
        let mut model = Model::new();
        model.insert(EntityId(7), Entity::new("IFCWALL", vec![Value::Null]));
        model.insert(EntityId(3), Entity::new("IFCSLAB", vec![Value::Null]));
        let mut buf = Vec::new();
        write(&model, &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        let wall = text.find("#7= IFCWALL").unwrap();
        let slab = text.find("#3= IFCSLAB").unwrap();
        assert!(wall < slab, "insertion order must be preserved");
    }
}

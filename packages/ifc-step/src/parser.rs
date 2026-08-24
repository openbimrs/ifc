//! Token stream to [`Model`] — the parsing stage.
//!
//! Builds entities from records. The parser deliberately understands *STEP
//! syntax only*: it never asks what an entity means, which is why an entity
//! type introduced in a future schema parses correctly here without any change.

use crate::error::StepError;
use crate::escape;
use crate::lexer::{Lexer, Token};
use ifc_model::header::Header;
use ifc_model::{Entity, EntityId, Model, Value};
use std::sync::Arc;

/// Parse a whole STEP physical file.
pub fn parse(input: &[u8]) -> Result<Model, StepError> {
    let mut model = Model::new();
    let mut lexer = Lexer::new(input);

    let mut section = Section::None;
    // Records look like `#id = NAME(args);` in DATA and `NAME(args);` in
    // HEADER, so the loop tracks a pending id rather than assuming one.
    let mut pending_id: Option<u64> = None;

    while let Some(token) = lexer.next_token() {
        match token {
            Token::Name(name) => {
                let upper = name.to_ascii_uppercase();
                match upper.as_slice() {
                    b"HEADER" => section = Section::Header,
                    b"DATA" => section = Section::Data,
                    b"ENDSEC" => section = Section::None,
                    // File magic and terminator: `ISO-10303-21;` and
                    // `END-ISO-10303-21;` lex as bare names with no argument
                    // list. Anything outside a section is structural noise.
                    _ if matches!(section, Section::None) => {}
                    _ => {
                        let args = parse_arguments(&mut lexer)?;
                        match section {
                            Section::Header => {
                                apply_header_entry(model.header_mut(), &upper, &args)
                            }
                            Section::Data => {
                                let id = pending_id.take().ok_or(StepError::MissingEntityId {
                                    offset: lexer.offset(),
                                })?;
                                let type_name: Arc<str> =
                                    String::from_utf8_lossy(&upper).into_owned().into();
                                model.insert(EntityId(id), Entity::new(type_name, args));
                            }
                            Section::None => {}
                        }
                    }
                }
            }
            Token::Id(id) => pending_id = Some(id),
            _ => {}
        }
    }
    Ok(model)
}

/// Which section the parser is inside.
enum Section {
    None,
    Header,
    Data,
}

/// Parse a parenthesised, comma-separated argument list.
///
/// Assumes the next token is `(`; returns the values in positional order.
fn parse_arguments(lexer: &mut Lexer<'_>) -> Result<Vec<Value>, StepError> {
    match lexer.next_token() {
        Some(Token::OpenParen) => {}
        _ => {
            return Err(StepError::Syntax {
                offset: lexer.offset(),
                detail: "expected '(' after entity type".into(),
            })
        }
    }
    parse_value_list(lexer)
}

/// Parse values until the matching `)`.
fn parse_value_list(lexer: &mut Lexer<'_>) -> Result<Vec<Value>, StepError> {
    let mut values = Vec::new();
    loop {
        match lexer.next_token() {
            None => {
                return Err(StepError::Syntax {
                    offset: lexer.offset(),
                    detail: "unterminated argument list".into(),
                })
            }
            Some(Token::CloseParen) => return Ok(values),
            Some(Token::Comma) => {}
            Some(token) => values.push(parse_value(lexer, token)?),
        }
    }
}

/// Parse one value, given the token that starts it.
fn parse_value(lexer: &mut Lexer<'_>, token: Token<'_>) -> Result<Value, StepError> {
    Ok(match token {
        Token::Dollar => Value::Null,
        Token::Star => Value::Derived,
        Token::Id(id) => Value::Ref(EntityId(id)),
        Token::Integer(i) => Value::Integer(i),
        Token::Real(r) => Value::Real(r),
        Token::Text(raw) => Value::Text(escape::decode(raw).into()),
        Token::Binary(raw) => Value::Binary(String::from_utf8_lossy(raw).into_owned().into()),
        Token::Keyword(kw) => match kw {
            b"T" => Value::Bool(true),
            b"F" => Value::Bool(false),
            b"U" => Value::LogicalUnknown,
            other => Value::Enum(String::from_utf8_lossy(other).into_owned().into()),
        },
        Token::OpenParen => Value::List(parse_value_list(lexer)?),
        // A bare name followed by `(` is a typed wrapper such as
        // IFCLENGTHMEASURE(2.5) or an inline aggregate type.
        Token::Name(name) => {
            let type_name: Arc<str> = String::from_utf8_lossy(name).into_owned().into();
            match lexer.next_token() {
                Some(Token::OpenParen) => {
                    let mut inner = parse_value_list(lexer)?;
                    let value = if inner.len() == 1 {
                        Box::new(inner.remove(0))
                    } else {
                        Box::new(Value::List(inner))
                    };
                    Value::Typed { type_name, value }
                }
                // A bare name with no parentheses: keep it as an enum-like
                // token rather than failing, so odd files still load.
                _ => Value::Enum(type_name),
            }
        }
        other => {
            return Err(StepError::Syntax {
                offset: lexer.offset(),
                detail: format!("unexpected token {other:?}"),
            })
        }
    })
}

/// Map a `HEADER;` record onto the model header.
fn apply_header_entry(header: &mut Header, name: &[u8], args: &[Value]) {
    fn text(v: Option<&Value>) -> String {
        v.and_then(|v| v.as_text()).unwrap_or_default().to_string()
    }
    fn texts(v: Option<&Value>) -> Vec<String> {
        v.and_then(|v| v.as_list())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|i| i.as_text().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    match name {
        b"FILE_DESCRIPTION" => {
            header.description = texts(args.first());
            header.implementation_level = text(args.get(1));
        }
        b"FILE_NAME" => {
            header.name = text(args.first());
            header.time_stamp = text(args.get(1));
            header.author = texts(args.get(2));
            header.organization = texts(args.get(3));
            header.preprocessor_version = text(args.get(4));
            header.originating_system = text(args.get(5));
            header.authorization = text(args.get(6));
        }
        b"FILE_SCHEMA" => header.schema = texts(args.first()),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('test.ifc','2026-01-01T00:00:00',('Author'),('Org'),'pre','sys','auth');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((0.0,1.5,-2.));
#2= IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',#1,'Wall',$,.ELEMENT.,#1,$,.T.);
#3= IFCPROPERTYSINGLEVALUE('Width',$,IFCLENGTHMEASURE(0.2),$);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn parses_header_fields() {
        let model = parse(SAMPLE).unwrap();
        assert_eq!(model.header().schema_token(), Some("IFC4"));
        assert_eq!(model.header().name, "test.ifc");
        assert_eq!(model.header().author, vec!["Author".to_string()]);
    }

    #[test]
    fn parses_entities_with_original_ids() {
        let model = parse(SAMPLE).unwrap();
        assert_eq!(model.len(), 3);
        assert!(model.get(EntityId(2)).unwrap().is_type("IfcWall"));
    }

    #[test]
    fn parses_nested_lists_and_reals() {
        let model = parse(SAMPLE).unwrap();
        let point = model.get(EntityId(1)).unwrap();
        let coords = point.attribute(0).unwrap().as_list().unwrap();
        assert_eq!(coords.len(), 3);
        assert_eq!(coords[2].as_f64(), Some(-2.0));
    }

    #[test]
    fn parses_typed_values() {
        let model = parse(SAMPLE).unwrap();
        let prop = model.get(EntityId(3)).unwrap();
        let value = prop.attribute(2).unwrap();
        match value {
            Value::Typed { type_name, .. } => assert_eq!(&**type_name, "IFCLENGTHMEASURE"),
            other => panic!("expected typed value, got {other:?}"),
        }
        assert_eq!(value.unwrap_typed().as_f64(), Some(0.2));
    }

    #[test]
    fn distinguishes_null_derived_bool_and_enum() {
        let model = parse(SAMPLE).unwrap();
        let wall = model.get(EntityId(2)).unwrap();
        assert_eq!(wall.attribute(3), Some(&Value::Null));
        assert_eq!(wall.attribute(4), Some(&Value::Enum("ELEMENT".into())));
        assert_eq!(wall.attribute(7), Some(&Value::Bool(true)));
    }

    #[test]
    fn indexes_by_type() {
        let model = parse(SAMPLE).unwrap();
        assert_eq!(model.ids_of_type("IFCWALL"), &[EntityId(2)]);
        assert_eq!(model.ids_of_type("ifcwall"), &[EntityId(2)]);
    }
}

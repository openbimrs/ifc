//! ifcXML text to [`Model`].
//!
//! Uses `quick-xml`'s pull parser: an IFC file can be very large, so the
//! document is never materialized as a tree.
//!
//! Unknown elements and attributes are preserved rather than rejected, on the
//! same principle as the STEP reader: a file containing entities from a
//! schema we do not know must still round-trip.

use crate::error::XmlError;
use crate::XmlCodec;
use ifc_model::{Entity, EntityId, Model, Value};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// Cheap sniff: does this look like an XML document?
pub fn looks_like_xml(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start_matches(['\u{feff}', ' ', '\n', '\r', '\t']);
    trimmed.starts_with("<?xml") || trimmed.starts_with("<ifcXML")
}

/// Parse an ifcXML document into a model.
pub fn read(_codec: &XmlCodec, bytes: &[u8]) -> Result<Model, XmlError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);

    let mut model = Model::new();
    let mut buf = Vec::new();

    // Header parsing state.
    let mut in_header = false;
    let mut header_tag: Option<String> = None;

    // Entity parsing state.
    let mut current: Option<PendingEntity> = None;
    // Stack of open child-value elements: (name, kind, type, accumulated items)
    let mut stack: Vec<PendingValue> = Vec::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(XmlError::Malformed(e.to_string())),
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "ifcXML" => {
                        if let Some(schema) = attr_value(&e, "schema") {
                            model.header_mut().schema = vec![schema];
                        }
                    }
                    "header" => in_header = true,
                    _ if in_header => {
                        header_tag = Some(name);
                        text_buf.clear();
                    }
                    _ if current.is_none() => {
                        // Opening an entity element.
                        if let Some(started) = start_entity(&e, &name) {
                            current = Some(started);
                        }
                    }
                    _ => {
                        // A child value element inside an entity.
                        stack.push(PendingValue::from_start(&e, name));
                        text_buf.clear();
                    }
                }
            }

            Ok(Event::Empty(e)) => {
                let name = local_name(&e);
                if current.is_none() && !in_header {
                    if let Some((id, type_name, attrs)) = start_entity(&e, &name) {
                        finish_entity(&mut model, id, type_name, attrs);
                    }
                } else if current.is_some() {
                    // Self-closing child: null or derived marker.
                    let value = if attr_value(&e, "nil").is_some() {
                        Value::Null
                    } else if attr_value(&e, "derived").is_some() {
                        Value::Derived
                    } else {
                        Value::Null
                    };
                    push_value(&mut stack, &mut current, name, value);
                }
            }

            Ok(Event::Text(t)) => {
                let raw = t
                    .unescape()
                    .map_err(|e| XmlError::Malformed(e.to_string()))?;
                text_buf.push_str(&raw);
            }

            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "header" => in_header = false,
                    "ifcXML" => {}
                    _ if in_header => {
                        if let Some(tag) = header_tag.take() {
                            apply_header_field(&mut model, &tag, &text_buf);
                        }
                        text_buf.clear();
                    }
                    _ => {
                        if let Some(pending) = stack.pop() {
                            let value = pending.finish(&text_buf);
                            push_value(&mut stack, &mut current, pending_name(&pending), value);
                            text_buf.clear();
                        } else if let Some((id, type_name, attrs)) = current.take() {
                            finish_entity(&mut model, id, type_name, attrs);
                        }
                    }
                }
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(model)
}

/// An entity being assembled: its id, type name, and named attributes.
///
/// Named rather than an inline tuple because it threads through four
/// functions; clippy flags the raw form as too complex, and it is right.
type PendingEntity = (EntityId, String, Vec<(String, Value)>);

/// A child element whose value is still being accumulated.
struct PendingValue {
    name: String,
    kind: String,
    type_name: Option<String>,
    items: Vec<Value>,
}

impl PendingValue {
    fn from_start(e: &BytesStart<'_>, name: String) -> Self {
        Self {
            name,
            kind: attr_value(e, "kind").unwrap_or_default(),
            type_name: attr_value(e, "type"),
            items: Vec::new(),
        }
    }

    fn finish(&self, text: &str) -> Value {
        match self.kind.as_str() {
            "list" => Value::List(self.items.clone()),
            "typed" => {
                let inner = self.items.first().cloned().unwrap_or(Value::Null);
                Value::Typed {
                    type_name: self.type_name.clone().unwrap_or_default().into(),
                    value: Box::new(inner),
                }
            }
            "enum" => Value::Enum(text.into()),
            "logical" => match text {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => Value::LogicalUnknown,
            },
            "binary" => Value::Binary(text.into()),
            "integer" => text.parse().map(Value::Integer).unwrap_or(Value::Null),
            "real" => text.parse().map(Value::Real).unwrap_or(Value::Null),
            "ref" => parse_ref(text).map(Value::Ref).unwrap_or(Value::Null),
            _ => Value::Text(text.into()),
        }
    }
}

fn pending_name(p: &PendingValue) -> String {
    p.name.clone()
}

/// Attach a finished value to its parent: an open list, or the entity.
fn push_value(
    stack: &mut [PendingValue],
    current: &mut Option<PendingEntity>,
    name: String,
    value: Value,
) {
    if let Some(parent) = stack.last_mut() {
        parent.items.push(value);
        return;
    }
    if let Some((_, _, attrs)) = current.as_mut() {
        attrs.push((name, value));
    }
}

/// Begin an entity element, reading its scalar attributes.
fn start_entity(e: &BytesStart<'_>, name: &str) -> Option<PendingEntity> {
    let id = attr_value(e, "id").and_then(|v| parse_ref(&v))?;
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
        if key == "id" {
            continue;
        }
        let value = attr
            .unescape_value()
            .map(|v| v.to_string())
            .unwrap_or_default();
        attrs.push((key, infer_scalar(&value)));
    }
    Some((id, name.to_string(), attrs))
}

/// Store a completed entity, ordering attributes by their positional name.
fn finish_entity(model: &mut Model, id: EntityId, type_name: String, attrs: Vec<(String, Value)>) {
    let mut ordered = attrs;
    // `a0`, `a1`, ... sort positionally; schema names keep document order.
    ordered.sort_by_key(|(name, _)| positional_index(name).unwrap_or(usize::MAX));
    let values: Vec<Value> = ordered.into_iter().map(|(_, v)| v).collect();
    model.insert(id, Entity::new(type_name, values));
}

/// `a12` -> `Some(12)`.
fn positional_index(name: &str) -> Option<usize> {
    name.strip_prefix('a')?.parse().ok()
}

/// `i42` -> `Some(EntityId(42))`.
fn parse_ref(text: &str) -> Option<EntityId> {
    let n: u64 = text.trim().strip_prefix('i')?.parse().ok()?;
    Some(EntityId(n))
}

/// Infer the kind of an attribute-encoded scalar.
///
/// Only unambiguous forms are promoted: `i<n>` is a reference, a valid integer
/// or real literal is numeric, everything else stays a string. Ambiguous cases
/// were written as child elements precisely so they never reach here.
fn infer_scalar(text: &str) -> Value {
    if let Some(id) = parse_ref(text) {
        return Value::Ref(id);
    }
    if let Ok(i) = text.parse::<i64>() {
        return Value::Integer(i);
    }
    if looks_real(text) {
        if let Ok(r) = text.parse::<f64>() {
            return Value::Real(r);
        }
    }
    Value::Text(text.into())
}

/// A STEP real always carries `.` or an exponent, which is what distinguishes
/// `1.` from the integer `1`.
fn looks_real(text: &str) -> bool {
    text.contains('.') || text.contains('e') || text.contains('E')
}

fn local_name(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).to_string()
}

fn attr_value(e: &BytesStart<'_>, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (a.key.local_name().as_ref() == key.as_bytes())
            .then(|| a.unescape_value().map(|v| v.to_string()).ok())
            .flatten()
    })
}

fn apply_header_field(model: &mut Model, tag: &str, text: &str) {
    let h = model.header_mut();
    match tag {
        "name" => h.name = text.to_string(),
        "time_stamp" => h.time_stamp = text.to_string(),
        "preprocessor_version" => h.preprocessor_version = text.to_string(),
        "originating_system" => h.originating_system = text.to_string(),
        "authorization" => h.authorization = text.to_string(),
        "author" => h.author.push(text.to_string()),
        "organization" => h.organization.push(text.to_string()),
        "description" => h.description.push(text.to_string()),
        _ => {}
    }
}

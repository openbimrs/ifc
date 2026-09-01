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
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Cheap sniff: does this look like an XML document?
pub fn looks_like_xml(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start_matches(['\u{feff}', ' ', '\n', '\r', '\t']);
    trimmed.starts_with("<?xml") || trimmed.starts_with("<ifcXML")
}

/// Parse an ifcXML document into a model.
pub fn read(codec: &XmlCodec, bytes: &[u8]) -> Result<Model, XmlError> {
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().trim_text(true);

    let mut model = Model::new();
    let mut buf = Vec::new();
    let mut seen_root = false;
    let mut root_closed = false;
    let mut element_depth = 0usize;

    // Header parsing state.
    let mut in_header = false;
    let mut header_tag: Option<String> = None;

    // Entity parsing state.
    let mut current: Option<PendingEntity> = None;
    // Stack of open child-value elements: (name, kind, type, accumulated items)
    let mut stack: Vec<PendingValue> = Vec::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_resolved_event_into(&mut buf) {
            Err(error) => {
                return Err(
                    XmlError::Malformed(error.to_string()).at(current_path(&current, &stack, None))
                );
            }
            Ok((_, Event::Eof)) => break,

            Ok((namespace, Event::Start(e))) => {
                let name = local_name(&e);
                validate_element(codec, namespace, &name)?;
                validate_root(codec, &e, &name, element_depth, root_closed, &mut seen_root)?;
                element_depth += 1;
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
                    _ if current.is_none() => match start_entity(&e, &name) {
                        Ok(Some(started)) => current = Some(started),
                        Ok(None) => {}
                        Err(error) => {
                            return Err(error.at(raw_entity_path(&name, attr_value(&e, "id"))));
                        }
                    },
                    _ => {
                        let item_index = stack.last().map_or(0, |parent| parent.items.len());
                        stack.push(PendingValue::from_start(&e, name, item_index));
                        text_buf.clear();
                    }
                }
            }

            Ok((namespace, Event::Empty(e))) => {
                let name = local_name(&e);
                validate_element(codec, namespace, &name)?;
                validate_root(codec, &e, &name, element_depth, root_closed, &mut seen_root)?;
                if codec.profile().is_some() && element_depth == 0 {
                    root_closed = true;
                }
                if current.is_none() && !in_header {
                    match start_entity(&e, &name) {
                        Ok(Some(entity)) => finish_entity(&mut model, entity),
                        Ok(None) => {}
                        Err(error) => {
                            return Err(error.at(raw_entity_path(&name, attr_value(&e, "id"))));
                        }
                    }
                } else if current.is_some() {
                    let value = if has_true_xsi_nil(&reader, &e) {
                        Value::Null
                    } else if attr_value(&e, "derived").as_deref() == Some("true") {
                        Value::Derived
                    } else {
                        let item_index = stack.last().map_or(0, |parent| parent.items.len());
                        let pending = PendingValue::from_start(&e, name.clone(), item_index);
                        let path =
                            current_path(&current, &stack, Some(pending.path_segment.as_str()));
                        pending.finish("").map_err(|error| error.at(path))?
                    };
                    push_value(&mut stack, &mut current, name, value);
                }
            }

            Ok((_, Event::Text(t))) => {
                let raw = t.unescape().map_err(|error| {
                    XmlError::Malformed(error.to_string()).at(current_path(&current, &stack, None))
                })?;
                text_buf.push_str(&raw);
            }

            Ok((namespace, Event::End(e))) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                validate_element(codec, namespace, &name)?;
                element_depth = element_depth.saturating_sub(1);
                if codec.profile().is_some() && element_depth == 0 {
                    root_closed = true;
                }
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
                            let path =
                                current_path(&current, &stack, Some(pending.path_segment.as_str()));
                            let value =
                                pending.finish(&text_buf).map_err(|error| error.at(path))?;
                            push_value(&mut stack, &mut current, pending.name.clone(), value);
                            text_buf.clear();
                        } else if let Some(entity) = current.take() {
                            finish_entity(&mut model, entity);
                        }
                    }
                }
            }
            Ok(_) => {}
        }
        buf.clear();
    }

    if codec.profile().is_some() && (!seen_root || !root_closed) {
        return Err(XmlError::Root { found: None });
    }

    Ok(model)
}

fn validate_element(
    codec: &XmlCodec,
    namespace: ResolveResult<'_>,
    element: &str,
) -> Result<(), XmlError> {
    let Some(profile) = codec.profile() else {
        return Ok(());
    };
    let found = match namespace {
        ResolveResult::Unbound => None,
        ResolveResult::Bound(namespace) => {
            Some(String::from_utf8_lossy(namespace.as_ref()).into_owned())
        }
        ResolveResult::Unknown(prefix) => Some(format!(
            "unresolved prefix `{}`",
            String::from_utf8_lossy(&prefix)
        )),
    };
    if found.as_deref() != Some(profile.namespace()) {
        return Err(XmlError::Namespace {
            element: element.into(),
            expected: profile.namespace(),
            found,
        });
    }
    Ok(())
}

fn validate_root(
    codec: &XmlCodec,
    element: &BytesStart<'_>,
    name: &str,
    depth: usize,
    root_closed: bool,
    seen_root: &mut bool,
) -> Result<(), XmlError> {
    let Some(profile) = codec.profile() else {
        return Ok(());
    };
    if *seen_root {
        if depth == 0 || root_closed || name == "ifcXML" {
            return Err(XmlError::Root {
                found: Some(name.into()),
            });
        }
        return Ok(());
    }
    if depth != 0 || name != "ifcXML" {
        return Err(XmlError::Root {
            found: Some(name.into()),
        });
    }
    *seen_root = true;
    let found = attr_value(element, "schema");
    if found.as_deref() != Some(profile.schema_token()) {
        return Err(XmlError::Profile {
            expected: profile.schema_token(),
            found,
        });
    }
    Ok(())
}

/// An entity being assembled: its id, type name, and named attributes.
///
/// Named rather than an inline tuple because it threads through four
/// functions; clippy flags the raw form as too complex, and it is right.
struct PendingEntity {
    id: EntityId,
    type_name: String,
    attrs: Vec<(String, Value)>,
}

/// A child element whose value is still being accumulated.
struct PendingValue {
    name: String,
    path_segment: String,
    kind: String,
    type_name: Option<String>,
    items: Vec<Value>,
}

impl PendingValue {
    fn from_start(e: &BytesStart<'_>, name: String, item_index: usize) -> Self {
        let path_segment = if name == "item" {
            format!("item[{item_index}]")
        } else {
            name.clone()
        };
        Self {
            name,
            path_segment,
            kind: attr_value(e, "kind").unwrap_or_default(),
            type_name: attr_value(e, "type"),
            items: Vec::new(),
        }
    }

    fn finish(&self, text: &str) -> Result<Value, XmlError> {
        let value = match self.kind.as_str() {
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
            "string" | "" => Value::Text(text.into()),
            "integer" => Value::Integer(text.parse().map_err(|_| invalid_scalar("integer", text))?),
            "real" => {
                let real: f64 = text.parse().map_err(|_| invalid_scalar("real", text))?;
                if !real.is_finite() {
                    return Err(invalid_scalar("real", text));
                }
                Value::Real(real)
            }
            "ref" => Value::Ref(parse_ref(text).ok_or_else(|| invalid_scalar("ref", text))?),
            kind => return Err(XmlError::UnknownKind(kind.into())),
        };
        Ok(value)
    }
}

fn invalid_scalar(kind: &str, value: &str) -> XmlError {
    XmlError::InvalidScalar {
        kind: kind.into(),
        value: value.into(),
    }
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
    if let Some(entity) = current.as_mut() {
        entity.attrs.push((name, value));
    }
}

/// Begin an entity element, reading its scalar attributes.
fn start_entity(e: &BytesStart<'_>, name: &str) -> Result<Option<PendingEntity>, XmlError> {
    let Some(id_text) = attr_value(e, "id") else {
        return Ok(None);
    };
    let id = parse_ref(&id_text).ok_or_else(|| XmlError::BadId(id_text.clone()))?;
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
        if key == "id" {
            continue;
        }
        let value = attr
            .unescape_value()
            .map(|value| value.to_string())
            .map_err(|error| XmlError::Malformed(error.to_string()))?;
        attrs.push((key, infer_scalar(&value)));
    }
    Ok(Some(PendingEntity {
        id,
        type_name: name.to_string(),
        attrs,
    }))
}

/// Store a completed entity, ordering attributes by their positional name.
fn finish_entity(model: &mut Model, entity: PendingEntity) {
    let mut ordered = entity.attrs;
    // `a0`, `a1`, ... sort positionally; schema names keep document order.
    ordered.sort_by_key(|(name, _)| positional_index(name).unwrap_or(usize::MAX));
    let values: Vec<Value> = ordered.into_iter().map(|(_, value)| value).collect();
    model.insert(entity.id, Entity::new(entity.type_name, values));
}

fn raw_entity_path(type_name: &str, id: Option<String>) -> String {
    match id {
        Some(id) => format!("/ifcXML/{type_name}[@id='{id}']"),
        None => format!("/ifcXML/{type_name}"),
    }
}

fn current_path(
    current: &Option<PendingEntity>,
    stack: &[PendingValue],
    leaf: Option<&str>,
) -> String {
    let mut path = String::from("/ifcXML");
    if let Some(entity) = current {
        path.push('/');
        path.push_str(&entity.type_name);
        path.push_str(&format!("[@id='i{}']", entity.id.0));
    }
    for value in stack {
        path.push('/');
        path.push_str(&value.path_segment);
    }
    if let Some(leaf) = leaf {
        path.push('/');
        path.push_str(leaf);
    }
    path
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

fn has_true_xsi_nil(reader: &NsReader<&[u8]>, element: &BytesStart<'_>) -> bool {
    element.attributes().flatten().any(|attribute| {
        if attribute.key.local_name().as_ref() != b"nil" {
            return false;
        }
        let (namespace, _) = reader.resolve_attribute(attribute.key);
        matches!(
            namespace,
            ResolveResult::Bound(namespace) if namespace.as_ref() == XSI_NAMESPACE.as_bytes()
        ) && attribute
            .unescape_value()
            .is_ok_and(|value| value.as_ref() == "true")
    })
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

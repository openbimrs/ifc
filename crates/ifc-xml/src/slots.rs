//! Attribute names to positional slots, owned in both directions.
//!
//! STEP attributes are positional; ifcXML names them. The writer names slot
//! `i` from the codec's schema when it has one and `a<i>` otherwise, and the
//! reader must map those names back to the same slots. XML attributes and
//! child elements arrive in two separate groups (scalars on the start tag,
//! structured values as children), so document order is *not* slot order and
//! only a name can place a value. Both directions resolve names through
//! [`schema_names`], so a schema-named document reads back into the order it
//! was written.

use crate::error::XmlError;
use crate::scalar::infer;
use crate::XmlCodec;
use ifc_model::Value;

/// A named value as the reader met it: an XML attribute's raw text, which
/// only the slot's declaration can type, or a child element's value, whose
/// kind the element spelled.
pub(crate) enum Raw {
    Text(String),
    Value(Value),
}

impl Raw {
    /// The value by inference: the lenient read, without a declaration.
    fn inferred(self) -> Value {
        match self {
            Self::Text(text) => infer(&text),
            Self::Value(value) => value,
        }
    }
}

/// Attribute names for a type: from the schema when available, else `a<i>`.
///
/// The fallback is deliberately obvious rather than a plausible guess: a wrong
/// name that looks right is worse than one that is visibly a placeholder.
pub(crate) fn attribute_names(codec: &XmlCodec, type_name: &str, count: usize) -> Vec<String> {
    let names = schema_names(codec, type_name);
    if names.len() >= count {
        return names.into_iter().take(count).collect();
    }
    (0..count).map(|slot| format!("a{slot}")).collect()
}

/// Order an entity's named values by slot.
///
/// A name resolves through the schema first, then as a positional `a<i>`.
/// Slots skipped below the schema's attribute count are omitted optional
/// attributes and read as unset; a positional name that skips a slot without
/// that bound is an error, as is a second value for one slot. Names that
/// resolve to no slot are unknown data and keep their document order after
/// the positioned values.
pub(crate) fn order(
    codec: &XmlCodec,
    type_name: &str,
    named: Vec<(String, Raw)>,
) -> Result<Vec<Value>, XmlError> {
    let names = schema_names(codec, type_name);
    let mut positioned: Vec<(usize, String, Value)> = Vec::new();
    let mut unknown = Vec::new();
    for (name, raw) in named {
        let value = raw.inferred();
        let slot = names
            .iter()
            .position(|candidate| *candidate == name)
            .or_else(|| positional_index(&name));
        match slot {
            Some(slot) => positioned.push((slot, name, value)),
            None => unknown.push(value),
        }
    }
    positioned.sort_by_key(|(slot, _, _)| *slot);

    let mut values = Vec::with_capacity(positioned.len() + unknown.len());
    for (slot, name, value) in positioned {
        if slot < values.len() {
            return Err(XmlError::DuplicateSlot { name, slot });
        }
        if slot > values.len() && slot > names.len() {
            return Err(XmlError::MissingSlot {
                name,
                slot,
                missing: values.len(),
            });
        }
        values.resize(slot, Value::Null);
        values.push(value);
    }
    values.extend(unknown);
    Ok(values)
}

/// The schema's attribute names for a type, or none without a usable schema.
///
/// A name list that repeats a name, or reuses the `id` / `a<i>` spellings the
/// codec reserves, cannot be written unambiguously, so it is not used.
#[cfg(feature = "schema")]
fn schema_names(codec: &XmlCodec, type_name: &str) -> Vec<String> {
    let Some(schema) = codec.schema() else {
        return Vec::new();
    };
    let names = schema.attribute_names(type_name);
    let mut seen = std::collections::HashSet::new();
    let usable = names
        .iter()
        .all(|name| *name != "id" && positional_index(name).is_none() && seen.insert(*name));
    if usable {
        names.into_iter().map(str::to_string).collect()
    } else {
        Vec::new()
    }
}

#[cfg(not(feature = "schema"))]
fn schema_names(codec: &XmlCodec, type_name: &str) -> Vec<String> {
    let _ = (codec, type_name);
    Vec::new()
}

/// `a12` -> `Some(12)`.
fn positional_index(name: &str) -> Option<usize> {
    name.strip_prefix('a')?.parse().ok()
}

/// Order and type an entity's named values strictly from the schema.
///
/// Every name must be an explicit attribute the entity declares, matched
/// exactly; an XML attribute's text is typed from that declaration, and a
/// child element's explicit kind must be one the declaration admits.
/// Nothing is inferred and nothing lands in a slot it does not name.
#[cfg(feature = "schema")]
pub(crate) fn order_strict(
    layouts: &mut crate::typing::Layouts<'_>,
    type_name: &str,
    path: &str,
    named: Vec<(String, Raw)>,
) -> Result<Vec<Value>, XmlError> {
    use crate::typing::{conform, scalar, Lexical};

    let layout = layouts
        .entity(type_name, false)?
        .ok_or_else(|| XmlError::UnknownEntity {
            name: type_name.into(),
        })?;
    if layout.abstract_ {
        return Err(XmlError::AbstractEntity {
            name: layout.name.to_string(),
        });
    }
    let mut slots: Vec<Option<Value>> = vec![None; layout.slots.len()];
    for (name, raw) in named {
        let slot = layout
            .slot(&name)
            .ok_or_else(|| XmlError::UnknownAttribute {
                entity: layout.name.to_string(),
                element: type_name.into(),
                attribute: name.clone(),
            })?;
        if slots[slot].is_some() {
            return Err(XmlError::DuplicateSlot { name, slot });
        }
        let shape = &layout.slots[slot].shape;
        let typed = match raw {
            Raw::Text(text) if shape.levels.is_empty() => {
                scalar(&shape.leaf, &text, Lexical::Native)
            }
            Raw::Text(_) => Err(XmlError::WrongForm {
                attribute: name.clone(),
                expected: "a list child element",
                found: "an XML attribute",
            }),
            Raw::Value(value) => Ok(value),
        };
        let value = typed
            .and_then(|value| conform(layouts.schema(), shape, &value).map(|()| value))
            .map_err(|error| error.at(format!("{path}/{name}")))?;
        slots[slot] = Some(value);
    }
    Ok(slots
        .into_iter()
        .map(|value| value.unwrap_or(Value::Null))
        .collect())
}

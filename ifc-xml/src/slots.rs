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
use crate::XmlCodec;
use ifc_model::Value;

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
    named: Vec<(String, Value)>,
) -> Result<Vec<Value>, XmlError> {
    let names = schema_names(codec, type_name);
    let mut positioned: Vec<(usize, String, Value)> = Vec::new();
    let mut unknown = Vec::new();
    for (name, value) in named {
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

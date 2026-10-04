//! The checks every record a batch writes passes, against the declared
//! release: `ifc-author`'s builder (names, types, forms, required and
//! derived slots), then what one record cannot settle alone -- that each
//! reference resolves to an entity of an accepted type, and that each
//! aggregate keeps its declared bounds.

use std::collections::HashSet;

use ifc_author::{AuthorError, EntityBuilder};
use ifc_model::{Entity, EntityId, Value};
use ifc_schema::{Aggregation, Attribute, Schema, TypeKind};

use super::overlay::Overlay;
use super::AuthoringFailure as F;

/// Build `type_name` from named values with `ifc-author`, against `schema`.
pub(super) fn build(
    schema: &Schema,
    type_name: &str,
    values: &[(String, Value)],
) -> Result<Entity, F> {
    let mut builder = EntityBuilder::new(schema, type_name);
    for (name, value) in values {
        builder = builder.set(name.clone(), value.clone());
    }
    builder.build().map_err(F::Author)
}

/// Re-check a whole record through `ifc-author`'s builder, as
/// `EntityEditor` checks an edited entity: every slot by its name.
pub(super) fn recheck(schema: &Schema, entity: &Entity) -> Result<(), F> {
    let declared = schema.attributes(&entity.type_name);
    if declared.len() != entity.attributes.len() {
        return Err(F::Author(AuthorError::ArityMismatch {
            entity: entity.type_name.to_string(),
            expected: declared.len(),
            found: entity.attributes.len(),
        }));
    }
    let named: Vec<(String, Value)> = declared
        .iter()
        .zip(&entity.attributes)
        .filter(|(_, value)| !matches!(value, Value::Null))
        .map(|(attribute, value)| (attribute.name.clone(), value.clone()))
        .collect();
    build(schema, &entity.type_name, &named).map(drop)
}

/// The references and aggregate bounds of `slots` (all when `None`).
pub(super) fn relations(
    schema: &Schema,
    view: &Overlay<'_>,
    entity: &Entity,
    slots: Option<&[usize]>,
) -> Result<(), F> {
    let declared = schema.attributes(&entity.type_name);
    for (slot, (attribute, value)) in declared.iter().zip(&entity.attributes).enumerate() {
        if slots.is_some_and(|only| !only.contains(&slot)) {
            continue;
        }
        bounds(&attribute.aggregation, value).map_err(|detail| F::Cardinality {
            entity: entity.type_name.to_string(),
            attribute: attribute.name.clone(),
            detail,
        })?;
        references(schema, view, &entity.type_name, attribute, value)?;
    }
    Ok(())
}

/// Every reference in `value` resolves, to an entity the declared type
/// accepts.
fn references(
    schema: &Schema,
    view: &Overlay<'_>,
    entity: &str,
    attribute: &Attribute,
    value: &Value,
) -> Result<(), F> {
    match value {
        Value::List(items) => items
            .iter()
            .try_for_each(|item| references(schema, view, entity, attribute, item)),
        Value::Ref(target) => {
            let Some(found) = view.get(*target) else {
                return Err(F::MissingReference {
                    entity: entity.to_owned(),
                    attribute: attribute.name.clone(),
                    target: *target,
                });
            };
            let declared = attribute.type_name.as_str();
            if checkable(schema, declared) && !schema.accepts_type(declared, &found.type_name) {
                return Err(F::WrongReferenceType {
                    entity: entity.to_owned(),
                    attribute: attribute.name.clone(),
                    target: *target,
                    actual: found.type_name.to_string(),
                    expected: declared.to_owned(),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Whether the tables settle which entities `declared` admits: an entity,
/// or a SELECT. A defined type aliasing an aggregate is not followed.
fn checkable(schema: &Schema, declared: &str) -> bool {
    schema.entity(declared).is_some()
        || schema
            .type_def(declared)
            .is_some_and(|definition| matches!(definition.kind, TypeKind::Select(_)))
}

/// The declared bounds and uniqueness of each aggregation level.
fn bounds(levels: &[Aggregation], value: &Value) -> Result<(), String> {
    let (Some(level), Value::List(items)) = (levels.first(), value) else {
        return Ok(());
    };
    let count = items.len() as u64;
    if let Some(lower) = level.lower.as_integer() {
        if count < lower {
            return Err(format!("{count} items; at least {lower}"));
        }
    }
    if let Some(upper) = level.upper.as_integer() {
        if count > upper {
            return Err(format!("{count} items; at most {upper}"));
        }
    }
    if level.forbids_duplicates() && has_duplicates(items) {
        return Err("a duplicate item where the aggregate is unique".to_owned());
    }
    items.iter().try_for_each(|item| bounds(&levels[1..], item))
}

/// Duplicate references always; other values in short aggregates, where
/// the pairwise comparison stays cheap.
fn has_duplicates(items: &[Value]) -> bool {
    let mut seen = HashSet::new();
    let refs = items.iter().all(|item| match item {
        Value::Ref(EntityId(id)) => {
            seen.insert(*id);
            true
        }
        _ => false,
    });
    if refs {
        return seen.len() != items.len();
    }
    items.len() <= 64
        && items
            .iter()
            .enumerate()
            .any(|(i, a)| items[i + 1..].contains(a))
}

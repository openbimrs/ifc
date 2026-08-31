//! Shared borrowed-view and strict positional decoding helpers.

use std::collections::HashSet;

use crate::{ClassificationError, ClassificationResult};
use ifc_model::{Entity, EntityId, Model, Value};

#[derive(Debug, Clone, Copy)]
pub struct ClassificationView<'m> {
    model: &'m Model,
}
impl<'m> ClassificationView<'m> {
    #[must_use]
    pub const fn new(model: &'m Model) -> Self {
        Self { model }
    }
    #[must_use]
    pub(crate) const fn model(self) -> &'m Model {
        self.model
    }
}

macro_rules! borrowed_entity {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            id: ifc_model::EntityId,
            entity: &'m ifc_model::Entity,
        }
        impl<'m> $name<'m> {
            pub fn try_new(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
            ) -> crate::ClassificationResult<Self> {
                if entity.is_type($kind) {
                    Ok(Self { id, entity })
                } else {
                    Err(crate::ClassificationError::WrongEntityType {
                        expected: $kind,
                        actual: entity.type_name.to_string(),
                    })
                }
            }
            pub(crate) const fn from_known(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
            ) -> Self {
                Self { id, entity }
            }
            #[must_use]
            pub const fn id(self) -> ifc_model::EntityId {
                self.id
            }
            #[must_use]
            pub const fn entity(self) -> &'m ifc_model::Entity {
                self.entity
            }
        }
    };
}
pub(crate) use borrowed_entity;

fn invalid(
    entity: &'static str,
    id: EntityId,
    attribute: &'static str,
    value: &Value,
) -> ClassificationError {
    ClassificationError::InvalidValue {
        entity,
        id,
        attribute,
        value: format!("{value:?}"),
    }
}

fn unique_refs(
    kind: &'static str,
    id: EntityId,
    attr: &'static str,
    values: &[Value],
) -> ClassificationResult<Vec<EntityId>> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let Value::Ref(target) = value else {
            return Err(invalid(kind, id, attr, value));
        };
        if !seen.insert(*target) {
            return Err(ClassificationError::InvalidValue {
                entity: kind,
                id,
                attribute: attr,
                value: format!("duplicate reference {target}"),
            });
        }
        out.push(*target);
    }
    Ok(out)
}
pub(crate) fn required_text<'m>(
    kind: &'static str,
    id: EntityId,
    e: &'m Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<&'m str> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Err(ClassificationError::MissingAttribute {
            entity: kind,
            id,
            attribute: attr,
        }),
        Some(v) => v
            .unwrap_typed()
            .as_text()
            .ok_or_else(|| invalid(kind, id, attr, v)),
    }
}
pub(crate) fn optional_text<'m>(
    kind: &'static str,
    id: EntityId,
    e: &'m Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<Option<&'m str>> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .unwrap_typed()
            .as_text()
            .map(Some)
            .ok_or_else(|| invalid(kind, id, attr, v)),
    }
}
pub(crate) fn optional_enum<'m>(
    kind: &'static str,
    id: EntityId,
    e: &'m Entity,
    slot: usize,
    attr: &'static str,
    allowed: &[&str],
) -> ClassificationResult<Option<&'m str>> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Enum(v)) if allowed.iter().any(|a| v.eq_ignore_ascii_case(a)) => Ok(Some(v)),
        Some(v) => Err(invalid(kind, id, attr, v)),
    }
}
pub(crate) fn optional_ref(
    kind: &'static str,
    id: EntityId,
    e: &Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<Option<EntityId>> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Ref(target)) => Ok(Some(*target)),
        Some(v) => Err(invalid(kind, id, attr, v)),
    }
}
pub(crate) fn required_ref(
    kind: &'static str,
    id: EntityId,
    e: &Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<EntityId> {
    optional_ref(kind, id, e, slot, attr)?.ok_or(ClassificationError::MissingAttribute {
        entity: kind,
        id,
        attribute: attr,
    })
}
pub(crate) fn required_refs(
    kind: &'static str,
    id: EntityId,
    e: &Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<Vec<EntityId>> {
    let value = e
        .attribute(slot)
        .ok_or(ClassificationError::MissingAttribute {
            entity: kind,
            id,
            attribute: attr,
        })?;
    let Value::List(values) = value else {
        return Err(invalid(kind, id, attr, value));
    };
    if values.is_empty() {
        return Err(invalid(kind, id, attr, value));
    }
    unique_refs(kind, id, attr, values)
}
pub(crate) fn optional_refs(
    kind: &'static str,
    id: EntityId,
    e: &Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<Option<Vec<EntityId>>> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::List(values)) if !values.is_empty() => {
            unique_refs(kind, id, attr, values).map(Some)
        }
        Some(v) => Err(invalid(kind, id, attr, v)),
    }
}
pub(crate) fn optional_texts<'m>(
    kind: &'static str,
    id: EntityId,
    e: &'m Entity,
    slot: usize,
    attr: &'static str,
) -> ClassificationResult<Option<Vec<&'m str>>> {
    match e.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::List(values)) if !values.is_empty() => values
            .iter()
            .map(|v| {
                v.unwrap_typed()
                    .as_text()
                    .ok_or_else(|| invalid(kind, id, attr, v))
            })
            .collect::<ClassificationResult<Vec<_>>>()
            .map(Some),
        Some(v) => Err(invalid(kind, id, attr, v)),
    }
}

//! Shared borrowed-view and strict positional decoding helpers.

use std::collections::HashSet;

use crate::{ClassificationError, ClassificationResult};
use ifc_model::{Entity, EntityId, Model, Value};

/// Entry point for classification/document/library queries over a borrowed [`Model`].
#[derive(Debug, Clone, Copy)]
pub struct ClassificationView<'m> {
    model: &'m Model,
}
impl<'m> ClassificationView<'m> {
    /// Borrow `model` for classification-schema queries.
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
        #[doc = concat!("Borrowed projection of an `", $kind, "` entity.")]
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            id: ifc_model::EntityId,
            entity: &'m ifc_model::Entity,
        }
        impl<'m> $name<'m> {
            #[doc = concat!(
                        "Project `entity` as `",
                        $kind,
                        "`, checking its runtime type; fails with `WrongEntityType` if it is not."
                    )]
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
            #[doc = concat!(
                        "Wrap `entity` as `",
                        $kind,
                        "` without re-checking its type; caller must already know it matches."
                    )]
            pub(crate) const fn from_known(
                id: ifc_model::EntityId,
                entity: &'m ifc_model::Entity,
            ) -> Self {
                Self { id, entity }
            }
            /// Entity id of the underlying instance.
            #[must_use]
            pub const fn id(self) -> ifc_model::EntityId {
                self.id
            }
            /// Borrowed underlying entity record.
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
/// Decode a required text attribute at `slot`; fails if unset or not text.
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
/// Decode an optional text attribute at `slot`; `None` when unset, fails if present but not text.
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
/// Decode an optional enumeration attribute at `slot`, restricted to `allowed`; fails on any other value.
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
/// Decode an optional single reference attribute at `slot`; `None` when unset, fails if present but not a reference.
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
/// Decode a required single reference attribute at `slot`; fails if unset or not a reference.
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
/// Decode a required, non-empty list of unique references at `slot`; fails if unset, empty, non-reference, or containing duplicates.
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
/// Decode an optional, non-empty list of unique references at `slot`; `None` when unset, fails on duplicates or non-references.
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
/// Decode an optional, non-empty list of text values at `slot`; `None` when unset, fails if any element is not text.
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

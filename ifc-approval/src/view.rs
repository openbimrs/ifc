//! Shared borrowed model view and strict positional decoders.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Value};

use crate::{ApprovalError, ApprovalResult};

/// Borrowed entry point for approval projections and inverse-style queries.
#[derive(Debug, Clone, Copy)]
pub struct ApprovalView<'m> {
    model: &'m Model,
}

impl<'m> ApprovalView<'m> {
    /// Borrow approval semantics from a model snapshot.
    #[must_use]
    pub const fn new(model: &'m Model) -> Self {
        Self { model }
    }

    pub(crate) const fn model(self) -> &'m Model {
        self.model
    }
}

pub(crate) fn wrong(expected: &'static str, entity: &Entity) -> ApprovalError {
    ApprovalError::WrongEntityType {
        expected,
        actual: entity.type_name.to_string(),
    }
}

fn invalid(
    kind: &'static str,
    id: EntityId,
    attribute: &'static str,
    value: &Value,
) -> ApprovalError {
    ApprovalError::InvalidValue {
        entity: kind,
        id,
        attribute,
        value: format!("{value:?}"),
    }
}

pub(crate) fn optional_text<'m>(
    kind: &'static str,
    id: EntityId,
    entity: &'m Entity,
    slot: usize,
    attribute: &'static str,
) -> ApprovalResult<Option<&'m str>> {
    match entity.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .unwrap_typed()
            .as_text()
            .map(Some)
            .ok_or_else(|| invalid(kind, id, attribute, value)),
    }
}

pub(crate) fn required_text<'m>(
    kind: &'static str,
    id: EntityId,
    entity: &'m Entity,
    slot: usize,
    attribute: &'static str,
) -> ApprovalResult<&'m str> {
    optional_text(kind, id, entity, slot, attribute)?.ok_or(ApprovalError::MissingAttribute {
        entity: kind,
        id,
        attribute,
    })
}

pub(crate) fn optional_ref(
    kind: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> ApprovalResult<Option<EntityId>> {
    match entity.attribute(slot) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Ref(target)) => Ok(Some(*target)),
        Some(value) => Err(invalid(kind, id, attribute, value)),
    }
}

pub(crate) fn required_ref(
    kind: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> ApprovalResult<EntityId> {
    optional_ref(kind, id, entity, slot, attribute)?.ok_or(ApprovalError::MissingAttribute {
        entity: kind,
        id,
        attribute,
    })
}

pub(crate) fn required_refs(
    kind: &'static str,
    id: EntityId,
    entity: &Entity,
    slot: usize,
    attribute: &'static str,
) -> ApprovalResult<Vec<EntityId>> {
    let value = entity
        .attribute(slot)
        .ok_or(ApprovalError::MissingAttribute {
            entity: kind,
            id,
            attribute,
        })?;
    let Value::List(values) = value else {
        return Err(invalid(kind, id, attribute, value));
    };
    if values.is_empty() {
        return Err(invalid(kind, id, attribute, value));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(values.len());
    for item in values {
        let Value::Ref(target) = item else {
            return Err(invalid(kind, id, attribute, item));
        };
        if !seen.insert(*target) {
            return Err(ApprovalError::InvalidValue {
                entity: kind,
                id,
                attribute,
                value: format!("duplicate {target}"),
            });
        }
        out.push(*target);
    }
    Ok(out)
}

pub(crate) fn validate_target(
    model: &Model,
    kind: &'static str,
    id: EntityId,
    attribute: &'static str,
    target: EntityId,
    expected: &'static str,
) -> ApprovalResult<()> {
    let actual = model.get(target).ok_or(ApprovalError::DanglingReference {
        entity: kind,
        id,
        attribute,
        target,
    })?;
    if ifc_schema::ifc4().accepts_type(expected, &actual.type_name) {
        Ok(())
    } else {
        Err(ApprovalError::ReferenceType {
            entity: kind,
            id,
            attribute,
            target,
            expected,
            actual: actual.type_name.to_string(),
        })
    }
}

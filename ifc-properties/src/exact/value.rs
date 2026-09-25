//! `IfcValue`/`IfcUnit` select acceptance and exact value resolution.

use std::collections::BTreeSet;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, TypeKind};

use super::release::Release;
use super::{ExactLogical, ExactPropertyError, ExactValue};

pub(super) fn select_accepts_type(schema: &Schema, select: &str, candidate: &str) -> bool {
    select_accepts(schema, select, candidate, false, &mut BTreeSet::new())
}

pub(super) fn select_accepts_entity(schema: &Schema, select: &str, candidate: &str) -> bool {
    select_accepts(schema, select, candidate, true, &mut BTreeSet::new())
}

pub(super) fn select_accepts(
    schema: &Schema,
    select: &str,
    candidate: &str,
    entity: bool,
    visited: &mut BTreeSet<String>,
) -> bool {
    let key = select.to_ascii_uppercase();
    if !visited.insert(key.clone()) {
        return false;
    }
    let accepted = schema.type_def(select).is_some_and(|definition| {
        let TypeKind::Select(members) = &definition.kind else {
            return false;
        };
        members.iter().any(|member| {
            member.eq_ignore_ascii_case(candidate)
                || (entity && schema.entity(member).is_some() && schema.is_a(candidate, member))
                || (schema.type_def(member).is_some()
                    && select_accepts(schema, member, candidate, entity, visited))
        })
    });
    visited.remove(&key);
    accepted
}

pub(super) fn typed_payload_matches(
    schema: &Schema,
    type_name: &str,
    value: &Value,
    visited: &mut BTreeSet<String>,
) -> bool {
    let key = type_name.to_ascii_uppercase();
    if !visited.insert(key.clone()) {
        return false;
    }
    let matches = schema
        .type_def(type_name)
        .is_some_and(|definition| match &definition.kind {
            TypeKind::Defined(rhs) => {
                let base = rhs
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric() || character == '_')
                    })
                    .find(|part| !part.is_empty())
                    .unwrap_or("");
                if schema.type_def(base).is_some() {
                    typed_payload_matches(schema, base, value, visited)
                } else {
                    match base.to_ascii_uppercase().as_str() {
                        "INTEGER" => matches!(value, Value::Integer(_)),
                        "REAL" => matches!(value, Value::Real(_)),
                        "NUMBER" => matches!(value, Value::Integer(_) | Value::Real(_)),
                        "STRING" => matches!(value, Value::Text(_)),
                        "BINARY" => matches!(value, Value::Binary(_)),
                        "BOOLEAN" => matches!(value, Value::Bool(_)),
                        "LOGICAL" => matches!(value, Value::Bool(_) | Value::LogicalUnknown),
                        _ => false,
                    }
                }
            }
            TypeKind::Enumeration(members) => match value {
                Value::Enum(member) => members
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(member)),
                _ => false,
            },
            TypeKind::Select(_) => match value {
                Value::Typed {
                    type_name: member,
                    value: payload,
                } if select_accepts_type(schema, type_name, member) => {
                    typed_payload_matches(schema, member, payload, visited)
                }
                _ => false,
            },
        });
    visited.remove(&key);
    matches
}

#[derive(Debug)]
pub(super) struct ResolvedValue {
    pub(super) value: ExactValue,
    pub(super) value_type: Option<Arc<str>>,
    pub(super) unit_id: Option<EntityId>,
}

pub(super) fn exact_property_value(
    model: &Model,
    release: Release,
    property: EntityId,
    entity: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
    let unit_id = match &entity.attributes[3] {
        Value::Null => None,
        Value::Ref(unit_id) => {
            let unit = model
                .get(*unit_id)
                .ok_or(ExactPropertyError::MissingReference {
                    from: property,
                    to: *unit_id,
                })?;
            if release.schema.entity(unit.type_name.as_ref()).is_none() {
                return Err(release.not_in_schema(*unit_id, unit.type_name.clone()));
            }
            if !select_accepts_entity(release.schema, "IFCUNIT", unit.type_name.as_ref()) {
                return Err(ExactPropertyError::UnsupportedUnit { property });
            }
            release.require_exact_slots(*unit_id, unit)?;
            Some(*unit_id)
        }
        _ => return Err(ExactPropertyError::UnsupportedUnit { property }),
    };
    match &entity.attributes[2] {
        Value::Null => Ok(ResolvedValue {
            value: ExactValue::Null,
            value_type: None,
            unit_id,
        }),
        // A name the release declares neither as a type nor as an entity is
        // foreign to it. A known name that `IfcValue` does not accept (an
        // entity such as `IfcOwnerHistory`) stays `UnsupportedValue` below.
        Value::Typed { type_name, .. }
            if release.schema.type_def(type_name).is_none()
                && release.schema.entity(type_name).is_none() =>
        {
            Err(release.not_in_schema(property, type_name.clone()))
        }
        Value::Typed { type_name, value }
            if select_accepts_type(release.schema, "IFCVALUE", type_name.as_ref())
                && typed_payload_matches(
                    release.schema,
                    type_name.as_ref(),
                    value.as_ref(),
                    &mut BTreeSet::new(),
                ) =>
        {
            exact_value(property, entity.attributes.get(2)).map(|value| ResolvedValue {
                value,
                value_type: Some(type_name.clone()),
                unit_id,
            })
        }
        _ => Err(ExactPropertyError::UnsupportedValue { property }),
    }
}

pub(super) fn exact_value(
    property: EntityId,
    value: Option<&Value>,
) -> Result<ExactValue, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MissingValueSlot { property });
    };
    match value {
        Value::Typed { type_name, value } if type_name.eq_ignore_ascii_case("IFCLOGICAL") => {
            match value.as_ref() {
                Value::Bool(false) => Ok(ExactValue::Logical(ExactLogical::False)),
                Value::LogicalUnknown => Ok(ExactValue::Logical(ExactLogical::Unknown)),
                Value::Bool(true) => Ok(ExactValue::Logical(ExactLogical::True)),
                _ => Err(ExactPropertyError::UnsupportedValue { property }),
            }
        }
        Value::Typed { value, .. } => exact_value(property, Some(value.as_ref())),
        Value::Null => Ok(ExactValue::Null),
        Value::Bool(v) => Ok(ExactValue::Bool(*v)),
        Value::LogicalUnknown => Ok(ExactValue::Logical(ExactLogical::Unknown)),
        Value::Binary(v) => Ok(ExactValue::Binary(v.clone())),
        Value::Integer(v) => Ok(ExactValue::Integer(*v)),
        Value::Real(v) if v.is_finite() => Ok(ExactValue::Real(*v)),
        Value::Real(_) => Err(ExactPropertyError::NonFiniteReal { property }),
        Value::Text(v) => Ok(ExactValue::Text(v.clone())),
        _ => Err(ExactPropertyError::UnsupportedValue { property }),
    }
}

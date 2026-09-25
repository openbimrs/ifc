//! Slot-count, reference, and aggregate readers shared by the traversal.

use std::collections::BTreeSet;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::release::Release;
use super::ExactPropertyError;

pub(super) fn require_exact_slots(
    schema: &Schema,
    entity_id: EntityId,
    entity: &Entity,
) -> Result<(), ExactPropertyError> {
    let expected = schema.attributes(entity.type_name.as_ref()).len();
    let actual = entity.attributes.len();
    if actual != expected {
        return Err(ExactPropertyError::MalformedEntitySlots {
            entity: entity_id,
            type_name: entity.type_name.clone(),
            expected,
            actual,
        });
    }
    Ok(())
}

pub(super) fn require_ref(
    model: &Model,
    from: EntityId,
    to: EntityId,
) -> Result<(), ExactPropertyError> {
    if model.get(to).is_some() {
        Ok(())
    } else {
        Err(ExactPropertyError::MissingReference { from, to })
    }
}
pub(super) fn property_definition_refs_at(
    release: Release,
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    match value {
        Value::Ref(id) => Ok(vec![*id]),
        // `IfcPropertySetDefinitionSet` is an IFC4 addition to the
        // `RelatingPropertyDefinition` select. Accept its typed or bare list
        // form only where the declared release defines it.
        Value::Typed { type_name, value }
            if type_name.eq_ignore_ascii_case("IFCPROPERTYSETDEFINITIONSET") =>
        {
            if release.schema.type_def(type_name).is_none() {
                return Err(release.not_in_schema(entity, type_name.clone()));
            }
            nonempty_refs_at(entity, Some(value.as_ref()), attribute)
        }
        Value::List(_) => {
            if release
                .schema
                .type_def("IFCPROPERTYSETDEFINITIONSET")
                .is_none()
            {
                return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
            }
            nonempty_refs_at(entity, Some(value), attribute)
        }
        _ => Err(ExactPropertyError::MalformedAggregate { entity, attribute }),
    }
}

pub(super) fn nonempty_refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let refs = refs_at(entity, value, attribute)?;
    if refs.is_empty() {
        Err(ExactPropertyError::MalformedAggregate { entity, attribute })
    } else {
        Ok(refs)
    }
}

pub(super) fn optional_refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    match value {
        None => Err(ExactPropertyError::MalformedAggregate { entity, attribute }),
        Some(Value::Null) => Ok(Vec::new()),
        value => nonempty_refs_at(entity, value, attribute),
    }
}
pub(super) fn refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    let Value::List(values) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|v| {
            let member = v
                .as_ref_id()
                .ok_or(ExactPropertyError::MalformedAggregate { entity, attribute })?;
            if !seen.insert(member) {
                return Err(ExactPropertyError::DuplicateAggregateMember {
                    entity,
                    attribute,
                    member,
                });
            }
            Ok(member)
        })
        .collect()
}
pub(super) fn ref_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<EntityId, ExactPropertyError> {
    value
        .and_then(Value::as_ref_id)
        .ok_or(ExactPropertyError::MalformedAggregate { entity, attribute })
}
pub(super) fn text_at<'a>(
    entity: EntityId,
    value: Option<&'a Value>,
    attribute: &'static str,
) -> Result<&'a str, ExactPropertyError> {
    value
        .and_then(|v| v.unwrap_typed().as_text())
        .ok_or(ExactPropertyError::MalformedName { entity, attribute })
}

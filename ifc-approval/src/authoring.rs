//! Transaction-staged authoring for the bounded approval domain.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::{ApprovalError, ApprovalResult};

const APPROVAL: &str = "IFCAPPROVAL";
const APPROVAL_REL: &str = "IFCAPPROVALRELATIONSHIP";
const RESOURCE_REL: &str = "IFCRESOURCEAPPROVALRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESAPPROVAL";

/// Draft for one `IfcApproval`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApprovalDraft<'a> {
    /// Optional identifier; at least this or `name` is required.
    pub identifier: Option<&'a str>,
    /// Optional name; at least this or `identifier` is required.
    pub name: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional IFC date-time lexical value.
    pub time_of_approval: Option<&'a str>,
    /// Optional status label.
    pub status: Option<&'a str>,
    /// Optional level label.
    pub level: Option<&'a str>,
    /// Optional qualifier text.
    pub qualifier: Option<&'a str>,
    /// Optional existing or earlier-staged `IfcActorSelect` target.
    pub requesting_approval: Option<EntityId>,
    /// Optional existing or earlier-staged `IfcActorSelect` target.
    pub giving_approval: Option<EntityId>,
}

/// Draft for one direct approval relationship.
#[derive(Debug, Clone, Copy)]
pub struct ApprovalRelationshipDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
    /// Non-empty unique related approvals.
    pub related_approvals: &'a [EntityId],
}

/// Draft for one approval-to-resource relationship.
#[derive(Debug, Clone, Copy)]
pub struct ResourceApprovalDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique `IfcResourceObjectSelect` targets.
    pub related_resources: &'a [EntityId],
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
}

/// Draft for one rooted approval association to definitions.
#[derive(Debug, Clone, Copy)]
pub struct ApprovalAssociationDraft<'a> {
    /// Compressed IFC GlobalId.
    pub global_id: &'a str,
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique `IfcDefinitionSelect` targets.
    pub related_objects: &'a [EntityId],
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
}

/// Validate and stage one approval.
pub fn create_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalDraft<'_>,
) -> ApprovalResult<EntityId> {
    if draft.identifier.is_none() && draft.name.is_none() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: APPROVAL,
            attribute: "WR",
            value: "Identifier and Name are both absent".into(),
        });
    }
    for target in [draft.requesting_approval, draft.giving_approval]
        .into_iter()
        .flatten()
    {
        validate_target(tx, model, target, "IfcActorSelect")?;
    }
    Ok(tx.create(Entity::new(
        APPROVAL,
        vec![
            optional_text(draft.identifier),
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.time_of_approval),
            optional_text(draft.status),
            optional_text(draft.level),
            optional_text(draft.qualifier),
            optional_ref(draft.requesting_approval),
            optional_ref(draft.giving_approval),
        ],
    )))
}

/// Validate and stage one approval-to-approval relationship.
pub fn relate_approvals(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalRelationshipDraft<'_>,
) -> ApprovalResult<EntityId> {
    validate_target(tx, model, draft.relating_approval, APPROVAL)?;
    validate_set(
        tx,
        model,
        APPROVAL_REL,
        "RelatedApprovals",
        draft.related_approvals,
        APPROVAL,
        Some(draft.relating_approval),
    )?;
    Ok(tx.create(Entity::new(
        APPROVAL_REL,
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            Value::Ref(draft.relating_approval),
            refs(draft.related_approvals),
        ],
    )))
}

/// Validate and stage one approval-to-resource relationship.
pub fn relate_resource_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ResourceApprovalDraft<'_>,
) -> ApprovalResult<EntityId> {
    validate_target(tx, model, draft.relating_approval, APPROVAL)?;
    validate_set(
        tx,
        model,
        RESOURCE_REL,
        "RelatedResourceObjects",
        draft.related_resources,
        "IfcResourceObjectSelect",
        None,
    )?;
    Ok(tx.create(Entity::new(
        RESOURCE_REL,
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            refs(draft.related_resources),
            Value::Ref(draft.relating_approval),
        ],
    )))
}

/// Validate and stage one rooted approval association.
pub fn associate_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalAssociationDraft<'_>,
) -> ApprovalResult<EntityId> {
    if Guid::parse(draft.global_id).is_none() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "GlobalId",
            value: draft.global_id.into(),
        });
    }
    validate_target(tx, model, draft.relating_approval, APPROVAL)?;
    validate_set(
        tx,
        model,
        ASSIGNMENT,
        "RelatedObjects",
        draft.related_objects,
        "IfcDefinitionSelect",
        None,
    )?;
    Ok(tx.create(Entity::new(
        ASSIGNMENT,
        vec![
            text(draft.global_id),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            refs(draft.related_objects),
            Value::Ref(draft.relating_approval),
        ],
    )))
}

fn validate_set(
    tx: &Transaction,
    model: &Model,
    kind: &'static str,
    attribute: &'static str,
    targets: &[EntityId],
    expected: &'static str,
    disallow: Option<EntityId>,
) -> ApprovalResult<()> {
    if targets.is_empty() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: kind,
            attribute,
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &target in targets {
        if !seen.insert(target) {
            return Err(ApprovalError::AuthoringInvalid {
                entity: kind,
                attribute,
                value: format!("duplicate {target}"),
            });
        }
        if Some(target) == disallow {
            return Err(ApprovalError::AuthoringInvalid {
                entity: kind,
                attribute,
                value: format!("self reference {target}"),
            });
        }
        validate_target(tx, model, target, expected)?;
    }
    Ok(())
}

fn validate_target(
    tx: &Transaction,
    model: &Model,
    target: EntityId,
    expected: &'static str,
) -> ApprovalResult<()> {
    let actual =
        final_type(tx, model, target).ok_or(ApprovalError::UnknownEntity { id: target })?;
    if ifc_schema::ifc4().accepts_type(expected, actual) {
        Ok(())
    } else {
        Err(ApprovalError::AuthoringReferenceType {
            target,
            expected,
            actual: actual.into(),
        })
    }
}

fn final_type<'a>(tx: &'a Transaction, model: &'a Model, id: EntityId) -> Option<&'a str> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(&entity.type_name),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.as_ref())
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}
fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
fn optional_ref(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}
fn refs(values: &[EntityId]) -> Value {
    Value::List(values.iter().copied().map(Value::Ref).collect())
}

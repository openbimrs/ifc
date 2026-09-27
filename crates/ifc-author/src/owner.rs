//! Authoring for `IfcOwnerHistory` and the actor records it points at.
//!
//! Every `IfcRoot` subtype carries `OwnerHistory` in slot 1. Domain crates in
//! this workspace currently write `Value::Null` there, which is schema-legal in
//! IFC4 but discards provenance: who authored a record, with which
//! application, and when. This module is the shared write side so each domain
//! crate can stop inventing its own.
//!
//! Ownership is deliberately NOT auto-attached. A caller that wants provenance
//! states it; one that does not keeps the existing null. Silently stamping an
//! invented actor onto every authored entity would be worse than an honest
//! omission, because downstream readers cannot tell a real author from a
//! placeholder.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::{AuthorError, AuthorResult};

/// Authored fields for `IfcPerson`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PersonDraft<'a> {
    /// `IfcPerson.Identification`.
    pub identification: Option<&'a str>,
    /// `IfcPerson.FamilyName`.
    pub family_name: Option<&'a str>,
    /// `IfcPerson.GivenName`.
    pub given_name: Option<&'a str>,
}

/// Authored fields for `IfcOrganization`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrganizationDraft<'a> {
    /// `IfcOrganization.Identification`.
    pub identification: Option<&'a str>,
    /// `IfcOrganization.Name`. Required by the schema.
    pub name: &'a str,
    /// `IfcOrganization.Description`.
    pub description: Option<&'a str>,
}

/// Authored fields for `IfcApplication`.
#[derive(Debug, Clone, Copy)]
pub struct ApplicationDraft<'a> {
    /// `IfcApplication.ApplicationDeveloper`, an `IfcOrganization`.
    pub developer: EntityId,
    /// `IfcApplication.Version`.
    pub version: &'a str,
    /// `IfcApplication.ApplicationFullName`.
    pub full_name: &'a str,
    /// `IfcApplication.ApplicationIdentifier`.
    pub identifier: &'a str,
}

/// Stage an `IfcPerson`.
///
/// IFC4 requires that a person carry at least one of Identification,
/// FamilyName or GivenName -- an entirely empty person identifies nobody and
/// defeats the purpose of recording ownership.
pub fn add_person(tx: &mut Transaction, draft: PersonDraft<'_>) -> AuthorResult<EntityId> {
    if draft.identification.is_none() && draft.family_name.is_none() && draft.given_name.is_none() {
        return Err(AuthorError::MissingRequired {
            entity: "IFCPERSON".to_owned(),
            attribute: "Identification|FamilyName|GivenName".to_owned(),
        });
    }
    Ok(tx.create(Entity::new(
        "IFCPERSON",
        vec![
            opt_text(draft.identification),
            opt_text(draft.family_name),
            opt_text(draft.given_name),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    )))
}

/// Stage an `IfcOrganization`. `Name` is required by the schema.
pub fn add_organization(
    tx: &mut Transaction,
    draft: OrganizationDraft<'_>,
) -> AuthorResult<EntityId> {
    if draft.name.trim().is_empty() {
        return Err(AuthorError::MissingRequired {
            entity: "IFCORGANIZATION".to_owned(),
            attribute: "Name".to_owned(),
        });
    }
    Ok(tx.create(Entity::new(
        "IFCORGANIZATION",
        vec![
            opt_text(draft.identification),
            Value::Text(draft.name.into()),
            opt_text(draft.description),
            Value::Null,
            Value::Null,
        ],
    )))
}

/// Stage an `IfcPersonAndOrganization`, the `IfcActorSelect` used by
/// `IfcOwnerHistory.OwningUser`.
pub fn add_person_and_organization(
    tx: &mut Transaction,
    person: EntityId,
    organization: EntityId,
) -> EntityId {
    tx.create(Entity::new(
        "IFCPERSONANDORGANIZATION",
        vec![Value::Ref(person), Value::Ref(organization), Value::Null],
    ))
}

/// Stage an `IfcApplication`, the authoring tool recorded in ownership.
pub fn add_application(
    tx: &mut Transaction,
    draft: ApplicationDraft<'_>,
) -> AuthorResult<EntityId> {
    for (attribute, value) in [
        ("Version", draft.version),
        ("ApplicationFullName", draft.full_name),
        ("ApplicationIdentifier", draft.identifier),
    ] {
        if value.trim().is_empty() {
            return Err(AuthorError::MissingRequired {
                entity: "IFCAPPLICATION".to_owned(),
                attribute: attribute.to_owned(),
            });
        }
    }
    Ok(tx.create(Entity::new(
        "IFCAPPLICATION",
        vec![
            Value::Ref(draft.developer),
            Value::Text(draft.version.into()),
            Value::Text(draft.full_name.into()),
            Value::Text(draft.identifier.into()),
        ],
    )))
}

/// Authored fields for `IfcOwnerHistory`.
#[derive(Debug, Clone, Copy)]
pub struct OwnerHistoryDraft<'a> {
    /// `IfcOwnerHistory.OwningUser`, an `IfcPersonAndOrganization`.
    pub owning_user: EntityId,
    /// `IfcOwnerHistory.OwningApplication`, an `IfcApplication`.
    pub owning_application: EntityId,
    /// `IfcOwnerHistory.ChangeAction`, an `IfcChangeActionEnum` constant.
    pub change_action: Option<&'a str>,
    /// `IfcOwnerHistory.CreationDate`, an IFC timestamp (seconds since the
    /// 1970 epoch).
    pub creation_date: i64,
    /// `IfcOwnerHistory.LastModifiedDate`, if the record was edited.
    pub last_modified_date: Option<i64>,
}

/// Valid `IfcChangeActionEnum` constants in IFC4 ADD2 TC1.
const CHANGE_ACTIONS: &[&str] = &["NOCHANGE", "MODIFIED", "ADDED", "DELETED", "NOTDEFINED"];

/// Stage an `IfcOwnerHistory`.
///
/// Two invariants are enforced here rather than left to a validator. A
/// `ChangeAction` outside `IfcChangeActionEnum` would serialize as an
/// unparseable enumeration token. And a `LastModifiedDate` earlier than
/// `CreationDate` describes a record edited before it existed: it parses,
/// validates, and quietly corrupts any audit trail built on it.
pub fn add_owner_history(
    tx: &mut Transaction,
    draft: OwnerHistoryDraft<'_>,
) -> AuthorResult<EntityId> {
    if let Some(action) = draft.change_action {
        if !CHANGE_ACTIONS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(action))
        {
            return Err(AuthorError::TypeMismatch {
                entity: "IFCOWNERHISTORY".to_owned(),
                attribute: "ChangeAction".to_owned(),
                expected: "an IfcChangeActionEnum constant".to_owned(),
                found: action.to_owned(),
            });
        }
    }
    if let Some(modified) = draft.last_modified_date {
        if modified < draft.creation_date {
            return Err(AuthorError::TypeMismatch {
                entity: "IFCOWNERHISTORY".to_owned(),
                attribute: "LastModifiedDate".to_owned(),
                expected: "a timestamp at or after CreationDate".to_owned(),
                found: modified.to_string(),
            });
        }
    }
    Ok(tx.create(Entity::new(
        "IFCOWNERHISTORY",
        vec![
            Value::Ref(draft.owning_user),
            Value::Ref(draft.owning_application),
            Value::Null,
            draft
                .change_action
                .map_or(Value::Null, |a| Value::Enum(a.into())),
            draft.last_modified_date.map_or(Value::Null, Value::Integer),
            Value::Null,
            Value::Null,
            Value::Integer(draft.creation_date),
        ],
    )))
}

fn opt_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

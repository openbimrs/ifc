//! Transactional IFC4 classification/document/library authoring.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::{ClassificationError, ClassificationResult};

#[derive(Debug, Clone, Copy)]
pub struct ClassificationDraft<'a> {
    pub source: Option<&'a str>,
    pub edition: Option<&'a str>,
    pub edition_date: Option<&'a str>,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub location: Option<&'a str>,
    pub reference_tokens: Option<&'a [&'a str]>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClassificationReferenceDraft<'a> {
    pub location: Option<&'a str>,
    pub identification: Option<&'a str>,
    pub name: Option<&'a str>,
    pub referenced_source: Option<EntityId>,
    pub description: Option<&'a str>,
    pub sort: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct DocumentDraft<'a> {
    pub identification: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub location: Option<&'a str>,
    pub purpose: Option<&'a str>,
    pub intended_use: Option<&'a str>,
    pub scope: Option<&'a str>,
    pub revision: Option<&'a str>,
    pub document_owner: Option<EntityId>,
    pub editors: Option<&'a [EntityId]>,
    pub creation_time: Option<&'a str>,
    pub last_revision_time: Option<&'a str>,
    pub electronic_format: Option<&'a str>,
    pub valid_from: Option<&'a str>,
    pub valid_until: Option<&'a str>,
    pub confidentiality: Option<&'a str>,
    pub status: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct DocumentReferenceDraft<'a> {
    pub location: Option<&'a str>,
    pub identification: Option<&'a str>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub referenced_document: Option<EntityId>,
}

#[derive(Debug, Clone, Copy)]
pub struct LibraryDraft<'a> {
    pub name: &'a str,
    pub version: Option<&'a str>,
    pub publisher: Option<EntityId>,
    pub version_date: Option<&'a str>,
    pub location: Option<&'a str>,
    pub description: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct LibraryReferenceDraft<'a> {
    pub location: Option<&'a str>,
    pub identification: Option<&'a str>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub language: Option<&'a str>,
    pub referenced_library: Option<EntityId>,
}

#[derive(Debug, Clone, Copy)]
pub struct AssociationDraft<'a> {
    pub global_id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub related_objects: &'a [EntityId],
}

pub(crate) fn text(value: &str) -> Value {
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
fn optional_enum(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Enum(Arc::from(v)))
}

pub(crate) fn final_type<'a>(
    tx: &'a Transaction,
    model: &'a Model,
    id: EntityId,
) -> Option<&'a str> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(&entity.type_name),
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.as_ref())
}

fn require_type(
    tx: &Transaction,
    model: &Model,
    id: EntityId,
    expected: &'static [&'static str],
    label: &'static str,
) -> ClassificationResult<()> {
    let actual = final_type(tx, model, id).ok_or(ClassificationError::UnknownEntity { id })?;
    if expected
        .iter()
        .any(|kind| actual.eq_ignore_ascii_case(kind))
    {
        return Ok(());
    }
    Err(ClassificationError::AuthoringReferenceType {
        target: id,
        expected: label,
        actual: actual.to_owned(),
    })
}

fn require_actor(tx: &Transaction, model: &Model, id: EntityId) -> ClassificationResult<()> {
    require_type(
        tx,
        model,
        id,
        &["IFCORGANIZATION", "IFCPERSON", "IFCPERSONANDORGANIZATION"],
        "IfcActorSelect",
    )
}

fn require_definition(tx: &Transaction, model: &Model, id: EntityId) -> ClassificationResult<()> {
    let actual = final_type(tx, model, id).ok_or(ClassificationError::UnknownEntity { id })?;
    let schema = ifc_schema::ifc4();
    if schema.is_a(actual, "IFCOBJECTDEFINITION") || schema.is_a(actual, "IFCPROPERTYDEFINITION") {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringReferenceType {
            target: id,
            expected: "IfcDefinitionSelect",
            actual: actual.to_owned(),
        })
    }
}

fn require_external_identity(
    entity: &'static str,
    location: Option<&str>,
    identification: Option<&str>,
    name: Option<&str>,
) -> ClassificationResult<()> {
    if location.is_some() || identification.is_some() || name.is_some() {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringInvalid {
            entity,
            attribute: "WR1",
            value: "Location, Identification, and Name are all unstated".into(),
        })
    }
}

fn require_enum(
    entity: &'static str,
    attribute: &'static str,
    value: Option<&str>,
    allowed: &[&str],
) -> ClassificationResult<()> {
    if let Some(value) = value {
        if !allowed
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(value))
        {
            return Err(ClassificationError::AuthoringInvalid {
                entity,
                attribute,
                value: value.into(),
            });
        }
    }
    Ok(())
}

pub fn create_classification(
    tx: &mut Transaction,
    draft: ClassificationDraft<'_>,
) -> ClassificationResult<EntityId> {
    if draft
        .reference_tokens
        .is_some_and(|tokens| tokens.is_empty())
    {
        return Err(ClassificationError::AuthoringInvalid {
            entity: "IFCCLASSIFICATION",
            attribute: "ReferenceTokens",
            value: "empty LIST [1:?]".into(),
        });
    }
    let tokens = draft.reference_tokens.map_or(Value::Null, |items| {
        Value::List(items.iter().map(|v| text(v)).collect())
    });
    Ok(tx.create(Entity::new(
        "IFCCLASSIFICATION",
        vec![
            optional_text(draft.source),
            optional_text(draft.edition),
            optional_text(draft.edition_date),
            text(draft.name),
            optional_text(draft.description),
            optional_text(draft.location),
            tokens,
        ],
    )))
}

pub fn create_classification_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: ClassificationReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    require_external_identity(
        "IFCCLASSIFICATIONREFERENCE",
        draft.location,
        draft.identification,
        draft.name,
    )?;
    if let Some(source) = draft.referenced_source {
        require_type(
            tx,
            model,
            source,
            &["IFCCLASSIFICATION", "IFCCLASSIFICATIONREFERENCE"],
            "IfcClassificationReferenceSelect",
        )?;
    }
    Ok(tx.create(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            optional_text(draft.location),
            optional_text(draft.identification),
            optional_text(draft.name),
            optional_ref(draft.referenced_source),
            optional_text(draft.description),
            optional_text(draft.sort),
        ],
    )))
}

pub fn create_document(
    tx: &mut Transaction,
    model: &Model,
    draft: DocumentDraft<'_>,
) -> ClassificationResult<EntityId> {
    require_enum(
        "IFCDOCUMENTINFORMATION",
        "Confidentiality",
        draft.confidentiality,
        &[
            "PUBLIC",
            "RESTRICTED",
            "CONFIDENTIAL",
            "PERSONAL",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    )?;
    require_enum(
        "IFCDOCUMENTINFORMATION",
        "Status",
        draft.status,
        &["DRAFT", "FINAL", "REVISION", "NOTDEFINED"],
    )?;
    if let Some(owner) = draft.document_owner {
        require_actor(tx, model, owner)?;
    }
    if let Some(editors) = draft.editors {
        if editors.is_empty() {
            return Err(ClassificationError::AuthoringInvalid {
                entity: "IFCDOCUMENTINFORMATION",
                attribute: "Editors",
                value: "empty SET [1:?]".into(),
            });
        }
        let mut seen = HashSet::new();
        for &editor in editors {
            if !seen.insert(editor) {
                return Err(ClassificationError::AuthoringInvalid {
                    entity: "IFCDOCUMENTINFORMATION",
                    attribute: "Editors",
                    value: format!("duplicate {editor}"),
                });
            }
            require_actor(tx, model, editor)?;
        }
    }
    let editors = draft.editors.map_or(Value::Null, refs);
    Ok(tx.create(Entity::new(
        "IFCDOCUMENTINFORMATION",
        vec![
            text(draft.identification),
            text(draft.name),
            optional_text(draft.description),
            optional_text(draft.location),
            optional_text(draft.purpose),
            optional_text(draft.intended_use),
            optional_text(draft.scope),
            optional_text(draft.revision),
            optional_ref(draft.document_owner),
            editors,
            optional_text(draft.creation_time),
            optional_text(draft.last_revision_time),
            optional_text(draft.electronic_format),
            optional_text(draft.valid_from),
            optional_text(draft.valid_until),
            optional_enum(draft.confidentiality),
            optional_enum(draft.status),
        ],
    )))
}

pub fn create_document_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: DocumentReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    require_external_identity(
        "IFCDOCUMENTREFERENCE",
        draft.location,
        draft.identification,
        draft.name,
    )?;
    if !(draft.name.is_some() ^ draft.referenced_document.is_some()) {
        return Err(ClassificationError::AuthoringInvalid {
            entity: "IFCDOCUMENTREFERENCE",
            attribute: "WR1",
            value: "exactly one of Name and ReferencedDocument must be stated".into(),
        });
    }
    if let Some(document) = draft.referenced_document {
        require_type(
            tx,
            model,
            document,
            &["IFCDOCUMENTINFORMATION"],
            "IfcDocumentInformation",
        )?;
    }
    Ok(tx.create(Entity::new(
        "IFCDOCUMENTREFERENCE",
        vec![
            optional_text(draft.location),
            optional_text(draft.identification),
            optional_text(draft.name),
            optional_text(draft.description),
            optional_ref(draft.referenced_document),
        ],
    )))
}

pub fn create_library(
    tx: &mut Transaction,
    model: &Model,
    draft: LibraryDraft<'_>,
) -> ClassificationResult<EntityId> {
    if let Some(publisher) = draft.publisher {
        require_actor(tx, model, publisher)?;
    }
    Ok(tx.create(Entity::new(
        "IFCLIBRARYINFORMATION",
        vec![
            text(draft.name),
            optional_text(draft.version),
            optional_ref(draft.publisher),
            optional_text(draft.version_date),
            optional_text(draft.location),
            optional_text(draft.description),
        ],
    )))
}

pub fn create_library_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: LibraryReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    require_external_identity(
        "IFCLIBRARYREFERENCE",
        draft.location,
        draft.identification,
        draft.name,
    )?;
    if let Some(library) = draft.referenced_library {
        require_type(
            tx,
            model,
            library,
            &["IFCLIBRARYINFORMATION"],
            "IfcLibraryInformation",
        )?;
    }
    Ok(tx.create(Entity::new(
        "IFCLIBRARYREFERENCE",
        vec![
            optional_text(draft.location),
            optional_text(draft.identification),
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.language),
            optional_ref(draft.referenced_library),
        ],
    )))
}

fn associate(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
    kind: &'static str,
    target_types: &'static [&'static str],
    label: &'static str,
) -> ClassificationResult<EntityId> {
    if Guid::parse(draft.global_id).is_none() {
        return Err(ClassificationError::AuthoringInvalid {
            entity: kind,
            attribute: "GlobalId",
            value: draft.global_id.into(),
        });
    }
    if draft.related_objects.is_empty() {
        return Err(ClassificationError::AuthoringInvalid {
            entity: kind,
            attribute: "RelatedObjects",
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &object in draft.related_objects {
        if !seen.insert(object) {
            return Err(ClassificationError::AuthoringInvalid {
                entity: kind,
                attribute: "RelatedObjects",
                value: format!("duplicate {object}"),
            });
        }
        require_definition(tx, model, object)?;
    }
    require_type(tx, model, target, target_types, label)?;
    Ok(tx.create(Entity::new(
        kind,
        vec![
            text(draft.global_id),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            refs(draft.related_objects),
            Value::Ref(target),
        ],
    )))
}

pub fn associate_classification(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    associate(
        tx,
        model,
        draft,
        target,
        "IFCRELASSOCIATESCLASSIFICATION",
        &["IFCCLASSIFICATION", "IFCCLASSIFICATIONREFERENCE"],
        "IfcClassificationSelect",
    )
}
pub fn associate_document(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    associate(
        tx,
        model,
        draft,
        target,
        "IFCRELASSOCIATESDOCUMENT",
        &["IFCDOCUMENTINFORMATION", "IFCDOCUMENTREFERENCE"],
        "IfcDocumentSelect",
    )
}
pub fn associate_library(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    associate(
        tx,
        model,
        draft,
        target,
        "IFCRELASSOCIATESLIBRARY",
        &["IFCLIBRARYINFORMATION", "IFCLIBRARYREFERENCE"],
        "IfcLibrarySelect",
    )
}

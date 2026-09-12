//! Transactional IFC4 classification/document/library authoring.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::{ClassificationError, ClassificationResult};

/// Draft for one IFC4 `IfcClassification`.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationDraft<'a> {
    /// `Source` publishing organization, when stated.
    pub source: Option<&'a str>,
    /// `Edition` identifier of the classification, when stated.
    pub edition: Option<&'a str>,
    /// `EditionDate` (IFC date string), when stated.
    pub edition_date: Option<&'a str>,
    /// Required `Name` of the classification system.
    pub name: &'a str,
    /// `Description` of the classification, when stated.
    pub description: Option<&'a str>,
    /// `Location` (URI) of the classification, when stated.
    pub location: Option<&'a str>,
    /// `ReferenceTokens` delimiter set; must be non-empty when given.
    pub reference_tokens: Option<&'a [&'a str]>,
}

/// Draft for one IFC4 `IfcClassificationReference`.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code within the classification, when stated.
    pub identification: Option<&'a str>,
    /// `Name` of the referenced item, when stated.
    pub name: Option<&'a str>,
    /// `ReferencedSource`: the parent `IfcClassification` or `IfcClassificationReference`, when stated.
    pub referenced_source: Option<EntityId>,
    /// `Description` of the reference, when stated.
    pub description: Option<&'a str>,
    /// `Sort` order token, when stated.
    pub sort: Option<&'a str>,
}

/// Draft for one IFC4 `IfcDocumentInformation`.
#[derive(Debug, Clone, Copy)]
pub struct DocumentDraft<'a> {
    /// Required `Identification` code of the document.
    pub identification: &'a str,
    /// Required `Name` of the document.
    pub name: &'a str,
    /// `Description` of the document, when stated.
    pub description: Option<&'a str>,
    /// `Location` (URI) of the document, when stated.
    pub location: Option<&'a str>,
    /// `Purpose` of the document, when stated.
    pub purpose: Option<&'a str>,
    /// `IntendedUse` of the document, when stated.
    pub intended_use: Option<&'a str>,
    /// `Scope` of the document, when stated.
    pub scope: Option<&'a str>,
    /// `Revision` identifier, when stated.
    pub revision: Option<&'a str>,
    /// `DocumentOwner`: an `IfcActorSelect`, when stated.
    pub document_owner: Option<EntityId>,
    /// `Editors`: non-empty unique set of `IfcActorSelect` ids, when stated.
    pub editors: Option<&'a [EntityId]>,
    /// `CreationTime` (IFC date-time string), when stated.
    pub creation_time: Option<&'a str>,
    /// `LastRevisionTime` (IFC date-time string), when stated.
    pub last_revision_time: Option<&'a str>,
    /// `ElectronicFormat`, when stated.
    pub electronic_format: Option<&'a str>,
    /// `ValidFrom` (IFC date string), when stated.
    pub valid_from: Option<&'a str>,
    /// `ValidUntil` (IFC date string), when stated.
    pub valid_until: Option<&'a str>,
    /// `Confidentiality` enumerator; must be one of the `IfcDocumentConfidentialityEnum` values.
    pub confidentiality: Option<&'a str>,
    /// `Status` enumerator; must be one of the `IfcDocumentStatusEnum` values.
    pub status: Option<&'a str>,
}

/// Draft for one IFC4 `IfcDocumentReference`.
#[derive(Debug, Clone, Copy)]
pub struct DocumentReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code, when stated.
    pub identification: Option<&'a str>,
    /// `Name`; exactly one of `name` and `referenced_document` must be given.
    pub name: Option<&'a str>,
    /// `Description` of the reference, when stated.
    pub description: Option<&'a str>,
    /// `ReferencedDocument`: an `IfcDocumentInformation`; exactly one of `name` and this must be given.
    pub referenced_document: Option<EntityId>,
}

/// Draft for one IFC4 `IfcLibraryInformation`.
#[derive(Debug, Clone, Copy)]
pub struct LibraryDraft<'a> {
    /// Required `Name` of the library.
    pub name: &'a str,
    /// `Version` identifier, when stated.
    pub version: Option<&'a str>,
    /// `Publisher`: an `IfcActorSelect`, when stated.
    pub publisher: Option<EntityId>,
    /// `VersionDate` (IFC date-time string), when stated.
    pub version_date: Option<&'a str>,
    /// `Location` (URI) of the library, when stated.
    pub location: Option<&'a str>,
    /// `Description` of the library, when stated.
    pub description: Option<&'a str>,
}

/// Draft for one IFC4 `IfcLibraryReference`.
#[derive(Debug, Clone, Copy)]
pub struct LibraryReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code within the library, when stated.
    pub identification: Option<&'a str>,
    /// `Name` of the referenced item, when stated.
    pub name: Option<&'a str>,
    /// `Description` of the reference, when stated.
    pub description: Option<&'a str>,
    /// `Language` of the referenced content, when stated.
    pub language: Option<&'a str>,
    /// `ReferencedLibrary`: an `IfcLibraryInformation`, when stated.
    pub referenced_library: Option<EntityId>,
}

/// Draft shared by `IfcRelAssociatesClassification`/`Document`/`Library`.
#[derive(Debug, Clone, Copy)]
pub struct AssociationDraft<'a> {
    /// Required `GlobalId`; must parse as a valid IFC GUID.
    pub global_id: &'a str,
    /// `Name` of the relationship, when stated.
    pub name: Option<&'a str>,
    /// `Description` of the relationship, when stated.
    pub description: Option<&'a str>,
    /// `RelatedObjects`: non-empty unique set of `IfcDefinitionSelect` ids.
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

/// Validate and stage one `IfcClassification`; fails if `reference_tokens` is `Some` and empty.
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

/// Validate and stage one `IfcClassificationReference`; fails if location/identification/name are all unstated or `referenced_source` does not resolve to an `IfcClassificationReferenceSelect`.
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

/// Validate and stage one `IfcDocumentInformation`; fails on an invalid `Confidentiality`/`Status` enumerator, a `document_owner`/`editors` entry that is not an `IfcActorSelect`, or a duplicate/empty `editors` set.
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

/// Validate and stage one `IfcDocumentReference`; fails if location/identification/name are all unstated, if `name` and `referenced_document` are not exactly one, or if `referenced_document` is not an `IfcDocumentInformation`.
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

/// Validate and stage one `IfcLibraryInformation`; fails if `publisher` does not resolve to an `IfcActorSelect`.
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

/// Validate and stage one `IfcLibraryReference`; fails if location/identification/name are all unstated or `referenced_library` is not an `IfcLibraryInformation`.
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

/// Validate and stage an `IfcRelAssociatesClassification` linking `draft.related_objects` to `target`; fails if `target` is not an `IfcClassificationSelect`, or on any shared association precondition.
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
/// Validate and stage an `IfcRelAssociatesDocument` linking `draft.related_objects` to `target`; fails if `target` is not an `IfcDocumentSelect`, or on any shared association precondition.
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
/// Validate and stage an `IfcRelAssociatesLibrary` linking `draft.related_objects` to `target`; fails if `target` is not an `IfcLibrarySelect`, or on any shared association precondition.
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

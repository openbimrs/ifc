//! Transactional classification/document/library authoring, bound to the
//! model's release.
//!
//! These helpers only stage records on a caller-owned [`Transaction`];
//! [`Transaction::commit`] owns atomic application.
//!
//! Every function that takes the [`Model`] lays its record out by attribute
//! name from the bundled table of the release the header declares (see
//! [`crate::classification_schema`]), never from IFC4 by assumption, and
//! checks every reference against that release's declared type. A record
//! type the release lacks (an IFC2X3 `IfcExternalReferenceRelationship`) is
//! refused with [`ClassificationError::EntityNotInSchema`]; a draft value
//! for an attribute it lacks (an IFC2X3 classification `Description`) with
//! [`ClassificationError::AuthoringNotInSchema`]; text for an attribute it
//! types as a record (an IFC2X3 `IfcCalendarDate`) with
//! [`ClassificationError::AuthoringValueType`]; and an attribute it
//! requires that the call leaves unset (the IFC2X3 `IfcRoot.OwnerHistory`,
//! or IFC2X3 `IfcClassification.Source`) with
//! [`ClassificationError::AuthoringRequired`]. A header binding no single
//! known release is refused with `MultipleSchemas` or `UnsupportedSchema`.
//! Nothing is staged on refusal.
//!
//! [`create_classification`] takes no model and cannot see the release: it
//! writes the IFC4 layout, which IFC4X3 shares. Use
//! [`create_classification_in`] for any other release.

mod association;
mod records;

pub use association::{
    associate_classification, associate_classification_with_owner_history, associate_document,
    associate_document_with_owner_history, associate_library, associate_library_with_owner_history,
    relate_documents,
};
pub use records::{
    create_classification, create_classification_in, create_classification_reference,
    create_document, create_document_reference, create_library, create_library_reference,
};

use std::sync::Arc;

use ifc_model::{Edit, EntityId, Model, Transaction, Value};

use crate::release::Release;
use crate::{ClassificationError, ClassificationResult};

/// Draft for one `IfcClassification`, fields named as IFC4 names them.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationDraft<'a> {
    /// `Source` publishing organization, when stated. Required by IFC2X3.
    pub source: Option<&'a str>,
    /// `Edition` identifier of the classification, when stated. Required by
    /// IFC2X3.
    pub edition: Option<&'a str>,
    /// `EditionDate` (IFC date string), when stated. IFC2X3 types it as an
    /// `IfcCalendarDate` record, so text is refused there.
    pub edition_date: Option<&'a str>,
    /// Required `Name` of the classification system.
    pub name: &'a str,
    /// `Description` of the classification, when stated. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `Location` (URI) of the classification, when stated: written to
    /// `Specification` in IFC4X3. IFC4 onwards.
    pub location: Option<&'a str>,
    /// `ReferenceTokens` delimiter set; must be non-empty when given. IFC4
    /// onwards.
    pub reference_tokens: Option<&'a [&'a str]>,
}

/// Draft for one `IfcClassificationReference`.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code within the classification, when stated
    /// (`ItemReference` in IFC2X3).
    pub identification: Option<&'a str>,
    /// `Name` of the referenced item, when stated.
    pub name: Option<&'a str>,
    /// `ReferencedSource`: the parent `IfcClassification` or
    /// `IfcClassificationReference` (only an `IfcClassification` in
    /// IFC2X3), when stated.
    pub referenced_source: Option<EntityId>,
    /// `Description` of the reference, when stated. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `Sort` order token, when stated. IFC4 onwards.
    pub sort: Option<&'a str>,
}

/// Draft for one `IfcDocumentInformation`.
#[derive(Debug, Clone, Copy)]
pub struct DocumentDraft<'a> {
    /// Required `Identification` code of the document (`DocumentId` in IFC2X3).
    pub identification: &'a str,
    /// Required `Name` of the document.
    pub name: &'a str,
    /// `Description` of the document, when stated.
    pub description: Option<&'a str>,
    /// `Location` (URI) of the document, when stated. IFC4 onwards.
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
    /// `CreationTime` (IFC date-time string), when stated. IFC2X3 types it
    /// as an `IfcDateAndTime` record, so text is refused there.
    pub creation_time: Option<&'a str>,
    /// `LastRevisionTime` (IFC date-time string), when stated. A record in
    /// IFC2X3, as `creation_time`.
    pub last_revision_time: Option<&'a str>,
    /// `ElectronicFormat`, when stated. IFC2X3 types it as an
    /// `IfcDocumentElectronicFormat` record, so text is refused there.
    pub electronic_format: Option<&'a str>,
    /// `ValidFrom` (IFC date string), when stated. An `IfcCalendarDate`
    /// record in IFC2X3.
    pub valid_from: Option<&'a str>,
    /// `ValidUntil` (IFC date string), when stated. An `IfcCalendarDate`
    /// record in IFC2X3.
    pub valid_until: Option<&'a str>,
    /// `Confidentiality` enumerator; must be one of the release's
    /// `IfcDocumentConfidentialityEnum` values.
    pub confidentiality: Option<&'a str>,
    /// `Status` enumerator; must be one of the release's
    /// `IfcDocumentStatusEnum` values.
    pub status: Option<&'a str>,
}

/// Draft for one `IfcDocumentReference`.
#[derive(Debug, Clone, Copy)]
pub struct DocumentReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code, when stated (`ItemReference` in IFC2X3).
    pub identification: Option<&'a str>,
    /// `Name`; exactly one of `name` and `referenced_document` must be given.
    pub name: Option<&'a str>,
    /// `Description` of the reference, when stated. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `ReferencedDocument`: an `IfcDocumentInformation`; exactly one of
    /// `name` and this must be given. IFC4 onwards: IFC2X3 links the other
    /// way, so there `name` is required.
    pub referenced_document: Option<EntityId>,
}

/// Draft for one `IfcLibraryInformation`.
#[derive(Debug, Clone, Copy)]
pub struct LibraryDraft<'a> {
    /// Required `Name` of the library.
    pub name: &'a str,
    /// `Version` identifier, when stated.
    pub version: Option<&'a str>,
    /// `Publisher`: an `IfcActorSelect` (only an `IfcOrganization` in
    /// IFC2X3), when stated.
    pub publisher: Option<EntityId>,
    /// `VersionDate` (IFC date-time string), when stated. An
    /// `IfcCalendarDate` record in IFC2X3.
    pub version_date: Option<&'a str>,
    /// `Location` (URI) of the library, when stated. IFC4 onwards.
    pub location: Option<&'a str>,
    /// `Description` of the library, when stated. IFC4 onwards.
    pub description: Option<&'a str>,
}

/// Draft for one `IfcLibraryReference`.
#[derive(Debug, Clone, Copy)]
pub struct LibraryReferenceDraft<'a> {
    /// `Location` (URI) of the reference; at least one of location/identification/name must be given.
    pub location: Option<&'a str>,
    /// `Identification` code within the library, when stated
    /// (`ItemReference` in IFC2X3).
    pub identification: Option<&'a str>,
    /// `Name` of the referenced item, when stated.
    pub name: Option<&'a str>,
    /// `Description` of the reference, when stated. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `Language` of the referenced content, when stated. IFC4 onwards.
    pub language: Option<&'a str>,
    /// `ReferencedLibrary`: an `IfcLibraryInformation`, when stated. IFC4
    /// onwards.
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
    /// `RelatedObjects`: non-empty unique set of object or property
    /// definitions (`IfcDefinitionSelect`; `IfcRoot` restricted by WR21 in
    /// IFC2X3).
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

/// The type `id` will have once `tx` commits: its last staged create or
/// retype, else the model's record. `None` when removed or absent.
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

/// Check that `target` (committed or staged) is a legal value of
/// `attribute` on `entity` in `release`, as the release declares it.
///
/// An attribute the release lacks is `AuthoringNotInSchema`; a target of
/// another type is `AuthoringReferenceType`, labelled with the release's
/// declared type (for example `IfcClassificationReferenceSelect`).
pub(crate) fn require_accepts(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    entity: &'static str,
    attribute: &'static str,
    target: EntityId,
) -> ClassificationResult<()> {
    let declared = release.declared(entity, attribute)?;
    let actual =
        final_type(tx, model, target).ok_or(ClassificationError::UnknownEntity { id: target })?;
    let (_, schema) = release.bound()?;
    if schema.accepts_type(&declared.type_name, actual) {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringReferenceType {
            target,
            expected: declared.type_name.as_str(),
            actual: actual.to_owned(),
        })
    }
}

/// Check `values` as a non-empty set of unique references accepted by
/// `attribute` on `entity` in `release`.
fn require_set(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    entity: &'static str,
    attribute: &'static str,
    values: &[EntityId],
) -> ClassificationResult<()> {
    if values.is_empty() {
        return Err(ClassificationError::AuthoringInvalid {
            entity,
            attribute,
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = std::collections::HashSet::new();
    for &value in values {
        if !seen.insert(value) {
            return Err(ClassificationError::AuthoringInvalid {
                entity,
                attribute,
                value: format!("duplicate {value}"),
            });
        }
        require_accepts(tx, model, release, entity, attribute, value)?;
    }
    Ok(())
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

/// Check an enumerator against the release's declared enumeration.
fn require_enum(
    release: Release<'_>,
    entity: &'static str,
    attribute: &'static str,
    value: Option<&str>,
) -> ClassificationResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let declared = release.declared(entity, attribute)?;
    let (_, schema) = release.bound()?;
    if crate::release::enumeration(schema, &declared.type_name)
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(value))
    {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringInvalid {
            entity,
            attribute,
            value: value.into(),
        })
    }
}

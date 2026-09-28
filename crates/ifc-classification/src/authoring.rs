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
#[non_exhaustive]
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

impl<'a> ClassificationDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str) -> Self {
        Self {
            source: None,
            edition: None,
            edition_date: None,
            name,
            description: None,
            location: None,
            reference_tokens: None,
        }
    }

    /// Sets `source`: `Source` publishing organization, when stated. Required
    /// by IFC2X3.
    #[must_use]
    pub fn source(mut self, value: &'a str) -> Self {
        self.source = Some(value);
        self
    }

    /// Sets `edition`: `Edition` identifier of the classification, when stated.
    /// Required by IFC2X3.
    #[must_use]
    pub fn edition(mut self, value: &'a str) -> Self {
        self.edition = Some(value);
        self
    }

    /// Sets `edition_date`: `EditionDate` (IFC date string), when stated.
    /// IFC2X3 types it as an `IfcCalendarDate` record, so text is refused
    /// there.
    #[must_use]
    pub fn edition_date(mut self, value: &'a str) -> Self {
        self.edition_date = Some(value);
        self
    }

    /// Sets `description`: `Description` of the classification, when stated.
    /// IFC4 onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the classification, when stated:
    /// written to `Specification` in IFC4X3. IFC4 onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `reference_tokens`: `ReferenceTokens` delimiter set; must be non-
    /// empty when given. IFC4 onwards.
    #[must_use]
    pub fn reference_tokens(mut self, value: &'a [&'a str]) -> Self {
        self.reference_tokens = Some(value);
        self
    }
}

/// Draft for one `IfcClassificationReference`.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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

impl<'a> ClassificationReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            referenced_source: None,
            description: None,
            sort: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code within the classification,
    /// when stated (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name` of the referenced item, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `referenced_source`: `ReferencedSource`: the parent
    /// `IfcClassification` or `IfcClassificationReference` (only an
    /// `IfcClassification` in IFC2X3), when stated.
    #[must_use]
    pub fn referenced_source(mut self, value: EntityId) -> Self {
        self.referenced_source = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `sort`: `Sort` order token, when stated. IFC4 onwards.
    #[must_use]
    pub fn sort(mut self, value: &'a str) -> Self {
        self.sort = Some(value);
        self
    }
}

/// Draft for one `IfcDocumentInformation`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> DocumentDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(identification: &'a str, name: &'a str) -> Self {
        Self {
            identification,
            name,
            description: None,
            location: None,
            purpose: None,
            intended_use: None,
            scope: None,
            revision: None,
            document_owner: None,
            editors: None,
            creation_time: None,
            last_revision_time: None,
            electronic_format: None,
            valid_from: None,
            valid_until: None,
            confidentiality: None,
            status: None,
        }
    }

    /// Sets `description`: `Description` of the document, when stated.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the document, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `purpose`: `Purpose` of the document, when stated.
    #[must_use]
    pub fn purpose(mut self, value: &'a str) -> Self {
        self.purpose = Some(value);
        self
    }

    /// Sets `intended_use`: `IntendedUse` of the document, when stated.
    #[must_use]
    pub fn intended_use(mut self, value: &'a str) -> Self {
        self.intended_use = Some(value);
        self
    }

    /// Sets `scope`: `Scope` of the document, when stated.
    #[must_use]
    pub fn scope(mut self, value: &'a str) -> Self {
        self.scope = Some(value);
        self
    }

    /// Sets `revision`: `Revision` identifier, when stated.
    #[must_use]
    pub fn revision(mut self, value: &'a str) -> Self {
        self.revision = Some(value);
        self
    }

    /// Sets `document_owner`: `DocumentOwner`: an `IfcActorSelect`, when
    /// stated.
    #[must_use]
    pub fn document_owner(mut self, value: EntityId) -> Self {
        self.document_owner = Some(value);
        self
    }

    /// Sets `editors`: `Editors`: non-empty unique set of `IfcActorSelect` ids,
    /// when stated.
    #[must_use]
    pub fn editors(mut self, value: &'a [EntityId]) -> Self {
        self.editors = Some(value);
        self
    }

    /// Sets `creation_time`: `CreationTime` (IFC date-time string), when
    /// stated. IFC2X3 types it as an `IfcDateAndTime` record, so text is
    /// refused there.
    #[must_use]
    pub fn creation_time(mut self, value: &'a str) -> Self {
        self.creation_time = Some(value);
        self
    }

    /// Sets `last_revision_time`: `LastRevisionTime` (IFC date-time string),
    /// when stated. A record in IFC2X3, as `creation_time`.
    #[must_use]
    pub fn last_revision_time(mut self, value: &'a str) -> Self {
        self.last_revision_time = Some(value);
        self
    }

    /// Sets `electronic_format`: `ElectronicFormat`, when stated. IFC2X3 types
    /// it as an `IfcDocumentElectronicFormat` record, so text is refused there.
    #[must_use]
    pub fn electronic_format(mut self, value: &'a str) -> Self {
        self.electronic_format = Some(value);
        self
    }

    /// Sets `valid_from`: `ValidFrom` (IFC date string), when stated. An
    /// `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn valid_from(mut self, value: &'a str) -> Self {
        self.valid_from = Some(value);
        self
    }

    /// Sets `valid_until`: `ValidUntil` (IFC date string), when stated. An
    /// `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn valid_until(mut self, value: &'a str) -> Self {
        self.valid_until = Some(value);
        self
    }

    /// Sets `confidentiality`: `Confidentiality` enumerator; must be one of the
    /// release's `IfcDocumentConfidentialityEnum` values.
    #[must_use]
    pub fn confidentiality(mut self, value: &'a str) -> Self {
        self.confidentiality = Some(value);
        self
    }

    /// Sets `status`: `Status` enumerator; must be one of the release's
    /// `IfcDocumentStatusEnum` values.
    #[must_use]
    pub fn status(mut self, value: &'a str) -> Self {
        self.status = Some(value);
        self
    }
}

/// Draft for one `IfcDocumentReference`.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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

impl<'a> DocumentReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            description: None,
            referenced_document: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code, when stated
    /// (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name`; exactly one of `name` and `referenced_document`
    /// must be given.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `referenced_document`: `ReferencedDocument`: an
    /// `IfcDocumentInformation`; exactly one of `name` and this must be given.
    /// IFC4 onwards: IFC2X3 links the other way, so there `name` is required.
    #[must_use]
    pub fn referenced_document(mut self, value: EntityId) -> Self {
        self.referenced_document = Some(value);
        self
    }
}

/// Draft for one `IfcLibraryInformation`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> LibraryDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            version: None,
            publisher: None,
            version_date: None,
            location: None,
            description: None,
        }
    }

    /// Sets `version`: `Version` identifier, when stated.
    #[must_use]
    pub fn version(mut self, value: &'a str) -> Self {
        self.version = Some(value);
        self
    }

    /// Sets `publisher`: `Publisher`: an `IfcActorSelect` (only an
    /// `IfcOrganization` in IFC2X3), when stated.
    #[must_use]
    pub fn publisher(mut self, value: EntityId) -> Self {
        self.publisher = Some(value);
        self
    }

    /// Sets `version_date`: `VersionDate` (IFC date-time string), when stated.
    /// An `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn version_date(mut self, value: &'a str) -> Self {
        self.version_date = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the library, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `description`: `Description` of the library, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Draft for one `IfcLibraryReference`.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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

impl<'a> LibraryReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            description: None,
            language: None,
            referenced_library: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code within the library, when
    /// stated (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name` of the referenced item, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `language`: `Language` of the referenced content, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn language(mut self, value: &'a str) -> Self {
        self.language = Some(value);
        self
    }

    /// Sets `referenced_library`: `ReferencedLibrary`: an
    /// `IfcLibraryInformation`, when stated. IFC4 onwards.
    #[must_use]
    pub fn referenced_library(mut self, value: EntityId) -> Self {
        self.referenced_library = Some(value);
        self
    }
}

/// Draft shared by `IfcRelAssociatesClassification`/`Document`/`Library`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> AssociationDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(global_id: &'a str, related_objects: &'a [EntityId]) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            related_objects,
        }
    }

    /// Sets `name`: `Name` of the relationship, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the relationship, when stated.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
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

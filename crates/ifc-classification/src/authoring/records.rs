//! Classification, document and library records, laid out by name in the
//! bound release.

use std::sync::Arc;

use ifc_model::{EntityId, Model, Transaction, Value};

use super::{
    optional_ref, optional_text, refs, require_accepts, require_enum, require_external_identity,
    require_set, text, ClassificationDraft, ClassificationReferenceDraft, DocumentDraft,
    DocumentReferenceDraft, LibraryDraft, LibraryReferenceDraft,
};
use crate::release::Release;
use crate::{ClassificationError, ClassificationResult};

fn optional_enum(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Enum(Arc::from(v)))
}

/// Validate and stage one `IfcClassification` in the IFC4 layout; fails if
/// `reference_tokens` is `Some` and empty.
///
/// Takes no model, so it cannot see the model's release: it writes the IFC4
/// ADD2 layout, which IFC4X3 ADD2 shares position for position (IFC4X3
/// names the sixth attribute `Specification`). For an IFC2X3 model, or to
/// have the header's release checked, use [`create_classification_in`].
pub fn create_classification(
    tx: &mut Transaction,
    draft: ClassificationDraft<'_>,
) -> ClassificationResult<EntityId> {
    stage_classification(tx, Release::LEGACY, draft)
}

/// Validate and stage one `IfcClassification` in the layout of `model`'s
/// declared release.
///
/// # Errors
///
/// Those of [`create_classification`], and for IFC2X3:
/// `AuthoringRequired` without `source` or `edition`, `AuthoringValueType`
/// for an `edition_date` (an `IfcCalendarDate` record there), and
/// `AuthoringNotInSchema` for a `description`, `location` or
/// `reference_tokens`. `MultipleSchemas` or `UnsupportedSchema` when the
/// header binds no single known release. Nothing is staged on an error.
pub fn create_classification_in(
    tx: &mut Transaction,
    model: &Model,
    draft: ClassificationDraft<'_>,
) -> ClassificationResult<EntityId> {
    stage_classification(tx, Release::of(model), draft)
}

fn stage_classification(
    tx: &mut Transaction,
    release: Release<'_>,
    draft: ClassificationDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCCLASSIFICATION";
    release.require_entity(ENTITY)?;
    if draft
        .reference_tokens
        .is_some_and(|tokens| tokens.is_empty())
    {
        return Err(ClassificationError::AuthoringInvalid {
            entity: ENTITY,
            attribute: "ReferenceTokens",
            value: "empty LIST [1:?]".into(),
        });
    }
    let tokens = draft.reference_tokens.map_or(Value::Null, |items| {
        Value::List(items.iter().map(|v| text(v)).collect())
    });
    let record = release.record(
        ENTITY,
        vec![
            ("Source", optional_text(draft.source)),
            ("Edition", optional_text(draft.edition)),
            ("EditionDate", optional_text(draft.edition_date)),
            ("Name", text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("Location", optional_text(draft.location)),
            ("ReferenceTokens", tokens),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one `IfcClassificationReference` in the layout of
/// `model`'s declared release.
///
/// # Errors
///
/// Location, identification and name all unstated (WR1); a
/// `referenced_source` the release does not accept
/// (`IfcClassificationReferenceSelect`, or only `IfcClassification` in
/// IFC2X3); a `description` or `sort` in IFC2X3 (`AuthoringNotInSchema`);
/// and a header binding no single known release. Nothing is staged on an
/// error.
pub fn create_classification_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: ClassificationReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCCLASSIFICATIONREFERENCE";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    require_external_identity(ENTITY, draft.location, draft.identification, draft.name)?;
    if let Some(source) = draft.referenced_source {
        require_accepts(tx, model, release, ENTITY, "ReferencedSource", source)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Location", optional_text(draft.location)),
            ("Identification", optional_text(draft.identification)),
            ("Name", optional_text(draft.name)),
            ("ReferencedSource", optional_ref(draft.referenced_source)),
            ("Description", optional_text(draft.description)),
            ("Sort", optional_text(draft.sort)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one `IfcDocumentInformation` in the layout of
/// `model`'s declared release.
///
/// # Errors
///
/// A `Confidentiality`/`Status` value outside the release's enumeration; a
/// `document_owner`/`editors` entry that is not an `IfcActorSelect`; a
/// duplicate or empty `editors` set; for IFC2X3, a `location`
/// (`AuthoringNotInSchema`) or a text date, time or electronic format, which
/// IFC2X3 types as records (`AuthoringValueType`); and a header binding no
/// single known release. Nothing is staged on an error.
pub fn create_document(
    tx: &mut Transaction,
    model: &Model,
    draft: DocumentDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCDOCUMENTINFORMATION";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    require_enum(release, ENTITY, "Confidentiality", draft.confidentiality)?;
    require_enum(release, ENTITY, "Status", draft.status)?;
    if let Some(owner) = draft.document_owner {
        require_accepts(tx, model, release, ENTITY, "DocumentOwner", owner)?;
    }
    if let Some(editors) = draft.editors {
        require_set(tx, model, release, ENTITY, "Editors", editors)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Identification", text(draft.identification)),
            ("Name", text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("Location", optional_text(draft.location)),
            ("Purpose", optional_text(draft.purpose)),
            ("IntendedUse", optional_text(draft.intended_use)),
            ("Scope", optional_text(draft.scope)),
            ("Revision", optional_text(draft.revision)),
            ("DocumentOwner", optional_ref(draft.document_owner)),
            ("Editors", draft.editors.map_or(Value::Null, refs)),
            ("CreationTime", optional_text(draft.creation_time)),
            ("LastRevisionTime", optional_text(draft.last_revision_time)),
            ("ElectronicFormat", optional_text(draft.electronic_format)),
            ("ValidFrom", optional_text(draft.valid_from)),
            ("ValidUntil", optional_text(draft.valid_until)),
            ("Confidentiality", optional_enum(draft.confidentiality)),
            ("Status", optional_enum(draft.status)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one `IfcDocumentReference` in the layout of `model`'s
/// declared release.
///
/// # Errors
///
/// Location, identification and name all unstated; `name` and
/// `referenced_document` not exactly one (WR1); a `referenced_document`
/// that is not an `IfcDocumentInformation`; for IFC2X3, a `description` or
/// `referenced_document` (`AuthoringNotInSchema`: IFC2X3 links a document
/// to its references from the other side, so a reference authored here
/// needs a `name`); and a header binding no single known release. Nothing
/// is staged on an error.
pub fn create_document_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: DocumentReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCDOCUMENTREFERENCE";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    require_external_identity(ENTITY, draft.location, draft.identification, draft.name)?;
    if !(draft.name.is_some() ^ draft.referenced_document.is_some()) {
        return Err(ClassificationError::AuthoringInvalid {
            entity: ENTITY,
            attribute: "WR1",
            value: "exactly one of Name and ReferencedDocument must be stated".into(),
        });
    }
    if let Some(document) = draft.referenced_document {
        require_accepts(tx, model, release, ENTITY, "ReferencedDocument", document)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Location", optional_text(draft.location)),
            ("Identification", optional_text(draft.identification)),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            (
                "ReferencedDocument",
                optional_ref(draft.referenced_document),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one `IfcLibraryInformation` in the layout of
/// `model`'s declared release.
///
/// # Errors
///
/// A `publisher` the release does not accept (`IfcActorSelect`, or only
/// `IfcOrganization` in IFC2X3); for IFC2X3, a `location` or `description`
/// (`AuthoringNotInSchema`) or a text `version_date` (`AuthoringValueType`);
/// and a header binding no single known release. Nothing is staged on an
/// error.
pub fn create_library(
    tx: &mut Transaction,
    model: &Model,
    draft: LibraryDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCLIBRARYINFORMATION";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    if let Some(publisher) = draft.publisher {
        require_accepts(tx, model, release, ENTITY, "Publisher", publisher)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Name", text(draft.name)),
            ("Version", optional_text(draft.version)),
            ("Publisher", optional_ref(draft.publisher)),
            ("VersionDate", optional_text(draft.version_date)),
            ("Location", optional_text(draft.location)),
            ("Description", optional_text(draft.description)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one `IfcLibraryReference` in the layout of `model`'s
/// declared release.
///
/// # Errors
///
/// Location, identification and name all unstated; a `referenced_library`
/// that is not an `IfcLibraryInformation`; for IFC2X3, a `description`,
/// `language` or `referenced_library` (`AuthoringNotInSchema`); and a
/// header binding no single known release. Nothing is staged on an error.
pub fn create_library_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: LibraryReferenceDraft<'_>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCLIBRARYREFERENCE";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    require_external_identity(ENTITY, draft.location, draft.identification, draft.name)?;
    if let Some(library) = draft.referenced_library {
        require_accepts(tx, model, release, ENTITY, "ReferencedLibrary", library)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Location", optional_text(draft.location)),
            ("Identification", optional_text(draft.identification)),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("Language", optional_text(draft.language)),
            ("ReferencedLibrary", optional_ref(draft.referenced_library)),
        ],
    )?;
    Ok(tx.create(record))
}

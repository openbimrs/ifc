//! `IfcRelAssociates*` and document relationships, laid out by name in the
//! bound release.
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from
//! IFC4 on:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! The `associate_*` writers leave it unset, so they refuse an IFC2X3 model
//! with [`ClassificationError::AuthoringRequired`] instead of writing `$`.
//! The `*_with_owner_history` variants take a caller-supplied
//! `IfcOwnerHistory` (in the model or staged on the transaction); one is
//! never invented here. This follows `ifc-material` (#77) and
//! `ifc-properties` (#191).

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use super::{optional_text, refs, require_accepts, text, AssociationDraft};
use crate::release::Release;
use crate::{ClassificationError, ClassificationResult};

/// Check `object` as a `RelatedObjects` member: the release's declared type
/// (`IfcDefinitionSelect`, or `IfcRoot` in IFC2X3) restricted to object and
/// property definitions (IFC2X3 `IfcRelAssociates.WR21`).
fn require_definition(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    relation: &'static str,
    object: EntityId,
) -> ClassificationResult<()> {
    let declared = release.declared(relation, "RelatedObjects")?;
    let actual = super::final_type(tx, model, object)
        .ok_or(ClassificationError::UnknownEntity { id: object })?;
    let (_, schema) = release.bound()?;
    let definition =
        schema.is_a(actual, "IFCOBJECTDEFINITION") || schema.is_a(actual, "IFCPROPERTYDEFINITION");
    if definition && schema.accepts_type(&declared.type_name, actual) {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringReferenceType {
            target: object,
            expected: declared.type_name.as_str(),
            actual: actual.to_owned(),
        })
    }
}

fn associate(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
    kind: &'static str,
    target_attribute: &'static str,
    owner_history: Option<EntityId>,
) -> ClassificationResult<EntityId> {
    let release = Release::of(model);
    release.require_entity(kind)?;
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
    let mut seen = std::collections::HashSet::new();
    for &object in draft.related_objects {
        if !seen.insert(object) {
            return Err(ClassificationError::AuthoringInvalid {
                entity: kind,
                attribute: "RelatedObjects",
                value: format!("duplicate {object}"),
            });
        }
        require_definition(tx, model, release, kind, object)?;
    }
    require_accepts(tx, model, release, kind, target_attribute, target)?;
    if let Some(owner_history) = owner_history {
        require_accepts(tx, model, release, kind, "OwnerHistory", owner_history)?;
    }
    let record = release.record(
        kind,
        vec![
            ("GlobalId", text(draft.global_id)),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatedObjects", refs(draft.related_objects)),
            (target_attribute, Value::Ref(target)),
        ],
    )?;
    Ok(tx.create(record))
}

const CLASSIFICATION: (&str, &str) = ("IFCRELASSOCIATESCLASSIFICATION", "RelatingClassification");
const DOCUMENT: (&str, &str) = ("IFCRELASSOCIATESDOCUMENT", "RelatingDocument");
const LIBRARY: (&str, &str) = ("IFCRELASSOCIATESLIBRARY", "RelatingLibrary");

/// Validate and stage an `IfcRelAssociatesClassification` linking
/// `draft.related_objects` to `target`, with `OwnerHistory` unset.
///
/// # Errors
///
/// A `target` the release's `RelatingClassification` does not accept
/// (`IfcClassificationSelect`; in IFC2X3 `IfcClassificationNotationSelect`,
/// which admits a reference or notation but no `IfcClassification`); a
/// malformed `GlobalId`; an empty, duplicated or non-definition
/// `RelatedObjects`; an IFC2X3 model, which requires `OwnerHistory`
/// (`AuthoringRequired`: use
/// [`associate_classification_with_owner_history`]); and a header binding
/// no single known release. Nothing is staged on an error.
pub fn associate_classification(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = CLASSIFICATION;
    associate(tx, model, draft, target, kind, attribute, None)
}

/// [`associate_classification`] with a caller-supplied `IfcOwnerHistory`,
/// which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`associate_classification`] except the IFC2X3 refusal, and an
/// `owner_history` that does not resolve (`UnknownEntity`) or is not an
/// `IfcOwnerHistory` (`AuthoringReferenceType`).
pub fn associate_classification_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
    owner_history: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = CLASSIFICATION;
    associate(
        tx,
        model,
        draft,
        target,
        kind,
        attribute,
        Some(owner_history),
    )
}

/// Validate and stage an `IfcRelAssociatesDocument` linking
/// `draft.related_objects` to `target`, with `OwnerHistory` unset.
///
/// # Errors
///
/// A `target` that is not an `IfcDocumentSelect`, the shared association
/// preconditions of [`associate_classification`], and an IFC2X3 model
/// (use [`associate_document_with_owner_history`]). Nothing is staged on an
/// error.
pub fn associate_document(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = DOCUMENT;
    associate(tx, model, draft, target, kind, attribute, None)
}

/// [`associate_document`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`associate_document`] except the IFC2X3 refusal, and the
/// owner-history refusals of [`associate_classification_with_owner_history`].
pub fn associate_document_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
    owner_history: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = DOCUMENT;
    associate(
        tx,
        model,
        draft,
        target,
        kind,
        attribute,
        Some(owner_history),
    )
}

/// Validate and stage an `IfcRelAssociatesLibrary` linking
/// `draft.related_objects` to `target`, with `OwnerHistory` unset.
///
/// # Errors
///
/// A `target` that is not an `IfcLibrarySelect`, the shared association
/// preconditions of [`associate_classification`], and an IFC2X3 model
/// (use [`associate_library_with_owner_history`]). Nothing is staged on an
/// error.
pub fn associate_library(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = LIBRARY;
    associate(tx, model, draft, target, kind, attribute, None)
}

/// [`associate_library`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`associate_library`] except the IFC2X3 refusal, and the
/// owner-history refusals of [`associate_classification_with_owner_history`].
pub fn associate_library_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: AssociationDraft<'_>,
    target: EntityId,
    owner_history: EntityId,
) -> ClassificationResult<EntityId> {
    let (kind, attribute) = LIBRARY;
    associate(
        tx,
        model,
        draft,
        target,
        kind,
        attribute,
        Some(owner_history),
    )
}

/// Stage an `IfcDocumentInformationRelationship` in the layout of `model`'s
/// declared release.
///
/// This is how a document says it supersedes, amends or accompanies
/// another. `RelationshipType` is a free label rather than an enum:
/// the schema does not fix the vocabulary, so it is written as given.
/// IFC2X3 declares only the three relationship attributes; IFC4 and IFC4X3
/// add the inherited `Name` and `Description`, left unset here.
///
/// # Errors
///
/// Refuses a reference that is not an `IfcDocumentInformation`, an
/// empty related set (`SET [1:?]`), a document related to itself, and a
/// header binding no single known release. Nothing is staged on an error.
pub fn relate_documents(
    tx: &mut Transaction,
    model: &Model,
    relating: EntityId,
    related: &[EntityId],
    relationship_type: Option<&str>,
) -> ClassificationResult<EntityId> {
    const ENTITY: &str = "IFCDOCUMENTINFORMATIONRELATIONSHIP";
    let release = Release::of(model);
    release.require_entity(ENTITY)?;
    require_accepts(tx, model, release, ENTITY, "RelatingDocument", relating)?;
    if related.is_empty() {
        return Err(ClassificationError::AuthoringInvalid {
            entity: ENTITY,
            attribute: "RelatedDocuments",
            value: "expected at least one document, per SET [1:?]".to_owned(),
        });
    }
    for document in related {
        require_accepts(tx, model, release, ENTITY, "RelatedDocuments", *document)?;
        // A document that supersedes itself is a cycle at depth one.
        if *document == relating {
            return Err(ClassificationError::AuthoringInvalid {
                entity: ENTITY,
                attribute: "RelatedDocuments",
                value: "expected a document other than the relating one".to_owned(),
            });
        }
    }
    let record = release.record(
        ENTITY,
        vec![
            ("RelatingDocument", Value::Ref(relating)),
            ("RelatedDocuments", refs(related)),
            ("RelationshipType", optional_text(relationship_type)),
        ],
    )?;
    Ok(tx.create(record))
}

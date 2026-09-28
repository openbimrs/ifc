//! #194: IFC2X3 authoring refuses, with a typed error and nothing staged,
//! every value and entity IFC2X3 TC1 cannot hold: attributes it lacks
//! (`AuthoringNotInSchema`), text where it declares a record
//! (`AuthoringValueType`), attributes it requires (`AuthoringRequired`,
//! including `IfcRoot.OwnerHistory`), references its selects do not admit,
//! and entities it does not declare (`EntityNotInSchema`).

mod common;

use common::{base, relation, ORGANIZATION, OWNER, PERSON, WALL};
use ifc_classification::{
    associate_classification, associate_classification_with_owner_history,
    create_classification_in, create_classification_reference, create_document,
    create_document_reference, create_external_reference_relationship, create_library,
    ClassificationDraft, ClassificationError, ClassificationReferenceDraft, DocumentDraft,
    DocumentReferenceDraft, ExternalReferenceRelationshipDraft, LibraryDraft, SchemaVersion,
};
use ifc_model::{EntityId, Transaction};

/// IFC2X3 refuses, with a typed error and nothing staged, every value and
/// entity it cannot hold.
#[test]
#[allow(clippy::too_many_lines)]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let v = SchemaVersion::Ifc2x3;
    let classification = ClassificationDraft::new("Uniclass")
        .source("NBS")
        .edition("2024");
    let refused = |result: Result<EntityId, ClassificationError>, tx: &Transaction| {
        assert!(tx.is_empty(), "a refusal stages nothing");
        result.expect_err("refused")
    };
    let not_in_schema = |entity, attribute| ClassificationError::AuthoringNotInSchema {
        entity,
        attribute,
        schema: v,
    };
    let result = create_classification_in(&mut tx, &model, classification.description("d"));
    assert_eq!(
        refused(result, &tx),
        not_in_schema("IFCCLASSIFICATION", "Description")
    );
    let result = create_classification_in(&mut tx, &model, classification.location("https://x"));
    assert_eq!(
        refused(result, &tx),
        not_in_schema("IFCCLASSIFICATION", "Location")
    );
    let result = create_classification_in(&mut tx, &model, {
        let mut draft = classification;
        draft.source = None;
        draft
    });
    assert_eq!(
        refused(result, &tx),
        ClassificationError::AuthoringRequired {
            entity: "IFCCLASSIFICATION",
            attribute: "Source",
            schema: v,
        }
    );
    let result =
        create_classification_in(&mut tx, &model, classification.edition_date("2024-01-01"));
    assert_eq!(
        refused(result, &tx),
        ClassificationError::AuthoringValueType {
            entity: "IFCCLASSIFICATION",
            attribute: "EditionDate",
            declared: "IfcCalendarDate",
            schema: v,
        }
    );
    let result = create_library(&mut tx, &model, LibraryDraft::new("L").publisher(PERSON));
    assert_eq!(
        refused(result, &tx),
        ClassificationError::AuthoringReferenceType {
            target: PERSON,
            expected: "IfcOrganization",
            actual: "IFCPERSON".to_owned(),
        }
    );
    let result = create_external_reference_relationship(
        &mut tx,
        &model,
        ExternalReferenceRelationshipDraft::new(WALL, &[ORGANIZATION]),
    );
    assert_eq!(
        refused(result, &tx),
        ClassificationError::EntityNotInSchema {
            entity: "IFCEXTERNALREFERENCERELATIONSHIP",
            schema: v,
        }
    );

    let document = DocumentDraft::new("DOC-1", "Spec");
    let result = create_document(&mut tx, &model, document.location("https://x"));
    assert_eq!(
        refused(result, &tx),
        not_in_schema("IFCDOCUMENTINFORMATION", "Location")
    );
    let result = create_document(
        &mut tx,
        &model,
        document.creation_time("2026-09-28T00:00:00"),
    );
    assert_eq!(
        refused(result, &tx),
        ClassificationError::AuthoringValueType {
            entity: "IFCDOCUMENTINFORMATION",
            attribute: "CreationTime",
            declared: "IfcDateAndTime",
            schema: v,
        }
    );
    let result = create_document(&mut tx, &model, document.status("APPROVED"));
    assert_eq!(
        refused(result, &tx),
        ClassificationError::AuthoringInvalid {
            entity: "IFCDOCUMENTINFORMATION",
            attribute: "Status",
            value: "APPROVED".to_owned(),
        }
    );

    // A system and a reference to associate, staged successfully.
    let system = create_classification_in(&mut tx, &model, classification).unwrap();
    let reference = create_classification_reference(
        &mut tx,
        &model,
        ClassificationReferenceDraft::new()
            .identification("Pr_20")
            .referenced_source(system),
    )
    .unwrap();
    let information = create_document(&mut tx, &model, document).unwrap();
    let staged = tx.len();
    let staged_after = |result: Result<EntityId, ClassificationError>, tx: &Transaction| {
        assert_eq!(tx.len(), staged, "a refusal stages nothing");
        result.expect_err("refused")
    };
    // IFC2X3 has no IfcDocumentReference.ReferencedDocument.
    let result = create_document_reference(
        &mut tx,
        &model,
        DocumentReferenceDraft::new()
            .identification("7")
            .referenced_document(information),
    );
    assert_eq!(
        staged_after(result, &tx),
        not_in_schema("IFCDOCUMENTREFERENCE", "ReferencedDocument")
    );
    // IFC2X3 has no reference-to-reference chains.
    let result = create_classification_reference(
        &mut tx,
        &model,
        ClassificationReferenceDraft::new()
            .identification("Pr_20_10")
            .referenced_source(reference),
    );
    assert_eq!(
        staged_after(result, &tx),
        ClassificationError::AuthoringReferenceType {
            target: reference,
            expected: "IfcClassification",
            actual: "IFCCLASSIFICATIONREFERENCE".to_owned(),
        }
    );
    let result = associate_classification(
        &mut tx,
        &model,
        relation("0YvctVUKr0kugbFTf53O08"),
        reference,
    );
    assert_eq!(
        staged_after(result, &tx),
        ClassificationError::AuthoringRequired {
            entity: "IFCRELASSOCIATESCLASSIFICATION",
            attribute: "OwnerHistory",
            schema: v,
        }
    );
    // IfcClassificationNotationSelect admits no IfcClassification.
    let result = associate_classification_with_owner_history(
        &mut tx,
        &model,
        relation("0YvctVUKr0kugbFTf53O08"),
        system,
        OWNER,
    );
    assert_eq!(
        staged_after(result, &tx),
        ClassificationError::AuthoringReferenceType {
            target: system,
            expected: "IfcClassificationNotationSelect",
            actual: "IFCCLASSIFICATION".to_owned(),
        }
    );
    let result = associate_classification_with_owner_history(
        &mut tx,
        &model,
        relation("0YvctVUKr0kugbFTf53O08"),
        reference,
        WALL,
    );
    assert_eq!(
        staged_after(result, &tx),
        ClassificationError::AuthoringReferenceType {
            target: WALL,
            expected: "IfcOwnerHistory",
            actual: "IFCWALL".to_owned(),
        }
    );
}

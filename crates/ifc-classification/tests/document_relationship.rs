//! Authoring `IfcDocumentInformationRelationship`.
//!
//! This is how a document says it supersedes or amends another.

use ifc_classification::{create_document, relate_documents, DocumentDraft};
use ifc_model::{Model, Transaction, Value};

fn document(tx: &mut Transaction, model: &Model, id: &str) -> ifc_model::EntityId {
    create_document(
        tx,
        model,
        DocumentDraft {
            identification: id,
            name: id,
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
        },
    )
    .expect("document")
}

/// A document relationship stages its five slots.
#[test]
fn a_document_relationship_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let current = document(&mut tx, &model, "SPEC-2");
    let superseded = document(&mut tx, &model, "SPEC-1");

    let id = relate_documents(&mut tx, &model, current, &[superseded], Some("SUPERSEDES"))
        .expect("relationship");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(
        staged.type_name.as_ref(),
        "IFCDOCUMENTINFORMATIONRELATIONSHIP",
    );
    assert_eq!(staged.attributes.len(), 5);
    assert_eq!(staged.attributes[2], Value::Ref(current));
    assert_eq!(
        staged.attributes[3],
        Value::List(vec![Value::Ref(superseded)])
    );
    assert_eq!(staged.attributes[4], Value::Text("SUPERSEDES".into()));
}

/// An empty set and a self-relation are both refused.
#[test]
fn a_degenerate_document_relationship_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let doc = document(&mut tx, &model, "SPEC-1");

    assert!(
        relate_documents(&mut tx, &model, doc, &[], None).is_err(),
        "an empty SET [1:?] was accepted",
    );
    assert!(
        relate_documents(&mut tx, &model, doc, &[doc], None).is_err(),
        "a document was related to itself",
    );
}

/// A reference that is not a document is refused.
#[test]
fn a_non_document_reference_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let doc = document(&mut tx, &model, "SPEC-1");
    let stray = tx.create(ifc_model::Entity::new(
        "IFCORGANIZATION",
        vec![Value::Null; 5],
    ));

    assert!(
        relate_documents(&mut tx, &model, doc, &[stray], None).is_err(),
        "an IfcOrganization was accepted as a document",
    );
}

//! #194: authoring writes the declared release's layout, by attribute name.
//!
//! Each release is authored through the public writers, committed, written
//! as STEP, read back with `ifc-step`, and re-read through the views. Every
//! written record has its release's arity, so no IFC4 layout lands in an
//! IFC2X3 model. `openbim-ifc`'s `release_bound_classification_authoring`
//! runs the same records through `ifc-validate`.

use ifc_classification::{
    associate_classification, associate_classification_with_owner_history, associate_document,
    associate_document_with_owner_history, associate_library, associate_library_with_owner_history,
    classification_schema, create_classification_in, create_classification_reference,
    create_document, create_document_reference, create_external_reference_relationship,
    create_library, create_library_reference, relate_documents, ClassificationDraft,
    ClassificationReferenceDraft, ClassificationView, DocumentDraft, DocumentReferenceDraft,
    ExternalReferenceRelationshipDraft, LibraryDraft, LibraryReferenceDraft, SchemaVersion,
};
use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_schema::for_version;

mod common;

use common::{base, relation, ORGANIZATION, OWNER, PERSON, RELEASES, WALL};

/// Author one record through every writer, with the values `version` can
/// hold; return the ids written.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let since_ifc4 = |value| ifc4.then_some(value);
    let mut tx = Transaction::new(model);
    let system = create_classification_in(&mut tx, model, {
        let mut draft = ClassificationDraft::new("Uniclass")
            .source("NBS")
            .edition("2024");
        draft.edition_date = since_ifc4("2024-01-01");
        draft.description = since_ifc4("Unified classification");
        draft.location = since_ifc4("https://uniclass.thenbs.com");
        draft.reference_tokens = ifc4.then_some(&["_"][..]);
        draft
    })
    .expect("classification");
    let reference = create_classification_reference(&mut tx, model, {
        let mut draft = ClassificationReferenceDraft::new()
            .location("https://uniclass.thenbs.com/Pr_20")
            .identification("Pr_20")
            .name("Products")
            .referenced_source(system);
        draft.description = since_ifc4("Structural");
        draft.sort = since_ifc4("20");
        draft
    })
    .expect("classification reference");
    let document_draft = |identification| {
        let mut draft = DocumentDraft::new(identification, "Spec")
            .description("Specification")
            .purpose("Tender")
            .revision("B")
            .document_owner(ORGANIZATION)
            .editors(&[PERSON])
            .confidentiality("PUBLIC")
            .status("FINALDRAFT");
        draft.location = since_ifc4("https://example.org/spec.pdf");
        draft.creation_time = since_ifc4("2026-09-28T00:00:00");
        draft.electronic_format = since_ifc4("application/pdf");
        draft.valid_from = since_ifc4("2026-09-28");
        draft
    };
    let document = create_document(&mut tx, model, document_draft("DOC-1")).expect("document");
    let superseded = create_document(&mut tx, model, document_draft("DOC-0")).expect("document");
    // IFC2X3 links a document to its references from the document side, so
    // a reference authored on its own carries a name instead.
    let document_ref = create_document_reference(&mut tx, model, {
        let mut draft = DocumentReferenceDraft::new().identification("7");
        draft.name = (!ifc4).then_some("Spec sheet");
        draft.description = since_ifc4("Sheet 7");
        draft.referenced_document = ifc4.then_some(document);
        draft
    })
    .expect("document reference");
    let library = create_library(&mut tx, model, {
        let mut draft = LibraryDraft::new("Products")
            .version("3")
            .publisher(ORGANIZATION);
        draft.version_date = since_ifc4("2026-09-28T00:00:00");
        draft.location = since_ifc4("https://example.org/lib");
        draft.description = since_ifc4("Product library");
        draft
    })
    .expect("library");
    let library_ref = create_library_reference(&mut tx, model, {
        let mut draft = LibraryReferenceDraft::new()
            .identification("P1")
            .name("Pump");
        draft.description = since_ifc4("Circulation pump");
        draft.language = since_ifc4("en");
        draft.referenced_library = ifc4.then_some(library);
        draft
    })
    .expect("library reference");
    let relationship =
        relate_documents(&mut tx, model, document, &[superseded], Some("SUPERSEDES"))
            .expect("document relationship");
    let (g1, g2, g3) = (
        "0YvctVUKr0kugbFTf53O08",
        "0YvctVUKr0kugbFTf53O09",
        "0YvctVUKr0kugbFTf53O0A",
    );
    let associations = if ifc4 {
        [
            associate_classification(&mut tx, model, relation(g1), reference),
            associate_document(&mut tx, model, relation(g2), document_ref),
            associate_library(&mut tx, model, relation(g3), library_ref),
        ]
    } else {
        [
            associate_classification_with_owner_history(
                &mut tx,
                model,
                relation(g1),
                reference,
                OWNER,
            ),
            associate_document_with_owner_history(
                &mut tx,
                model,
                relation(g2),
                document_ref,
                OWNER,
            ),
            associate_library_with_owner_history(&mut tx, model, relation(g3), library_ref, OWNER),
        ]
    }
    .map(|written| written.expect("association"));
    let mut written = vec![
        system,
        reference,
        document,
        superseded,
        document_ref,
        library,
        library_ref,
        relationship,
    ];
    written.extend(associations);
    if ifc4 {
        written.push(
            create_external_reference_relationship(
                &mut tx,
                model,
                ExternalReferenceRelationshipDraft::new(library_ref, &[ORGANIZATION]),
            )
            .expect("external reference relationship"),
        );
    }
    tx.commit(model).expect("commit");
    written
}

fn round_trip(model: &Model) -> Model {
    let bytes = ifc_step::StepCodec.write_bytes(model).expect("written");
    let back = ifc_step::StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

#[test]
fn authored_records_round_trip_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        let back = round_trip(&model);
        assert_eq!(classification_schema(&back), Ok(version), "{schema}");

        let table = for_version(version).unwrap();
        for id in &written {
            let record = back.get(*id).expect("written record");
            assert_eq!(
                record.attributes.len(),
                table.attributes(&record.type_name).len(),
                "{schema}: {} has its release's arity",
                record.type_name
            );
        }

        let view = ClassificationView::new(&back);
        let assignments = view.classification_assignments_for(WALL).unwrap();
        assert_eq!(assignments.len(), 1, "{schema}");
        let reference = view.references().next().unwrap();
        assert_eq!(reference.identification(), Ok(Some("Pr_20")), "{schema}");
        let hierarchy = view
            .hierarchy_from(reference.id(), ifc_model::Budget::default())
            .unwrap();
        let system = hierarchy.system.expect("classification root");
        assert_eq!(system.name(), Ok("Uniclass"));
        assert_eq!(system.source(), Ok(Some("NBS")));
        assert_eq!(view.document_assignments_for(WALL).unwrap().len(), 1);
        assert_eq!(view.library_assignments_for(WALL).unwrap().len(), 1);
        let document = view.documents().next().unwrap();
        assert_eq!(document.identification(), Ok("DOC-1"), "{schema}");
        assert_eq!(document.status(), Ok(Some("FINALDRAFT")), "{schema}");
        let document_ref = view.document_references().next().unwrap();
        assert_eq!(document_ref.identification(), Ok(Some("7")), "{schema}");
        let library_ref = view.library_references().next().unwrap();
        assert_eq!(library_ref.identification(), Ok(Some("P1")), "{schema}");

        let owner = &back.get(assignments[0].id()).unwrap().attributes[1];
        match version {
            SchemaVersion::Ifc2x3 => {
                assert_eq!(owner, &Value::Ref(OWNER));
                assert_eq!(system.description().map_err(|_| ()), Err(()));
            }
            _ => {
                assert_eq!(owner, &Value::Null);
                assert_eq!(system.location(), Ok(Some("https://uniclass.thenbs.com")));
                assert_eq!(view.external_references_for(ORGANIZATION).unwrap().len(), 1);
            }
        }
    }
}

//! Classification, document and library records authored in IFC2X3, IFC4
//! and IFC4X3 validate against their own release (#194).
//!
//! Each release is authored through the public writers of
//! `ifc-classification`, written to STEP, read back with `ifc-step`, and
//! checked by `ifc-validate` against the declared release's table. No record
//! this test wrote may carry an error finding. IFC2X3 requires
//! `IfcRoot.OwnerHistory`, so its associations use the
//! `*_with_owner_history` writers; IFC4 and IFC4X3 the writers that leave it
//! unset, which those releases allow.

#![cfg(all(
    feature = "validate",
    feature = "classification",
    feature = "schema",
    feature = "step"
))]

use ifc::classification::{
    associate_classification, associate_classification_with_owner_history, associate_document,
    associate_document_with_owner_history, associate_library, associate_library_with_owner_history,
    create_classification_in, create_classification_reference, create_document,
    create_document_reference, create_external_reference_relationship, create_library,
    create_library_reference, relate_documents, AssociationDraft, ClassificationDraft,
    ClassificationReferenceDraft, DocumentDraft, DocumentReferenceDraft,
    ExternalReferenceRelationshipDraft, LibraryDraft, LibraryReferenceDraft, SchemaVersion,
};
use ifc::schema::for_version;
use ifc::{Codec, Model, StepCodec, Value};
use ifc_model::{EntityId, Transaction};

const PERSON: EntityId = EntityId(1);
const ORGANIZATION: EntityId = EntityId(2);
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);

/// Actors, an owner history (`#5`) and a wall (`#10`) in `schema`.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let unset = ",$".repeat(for_version(version).unwrap().attributes("IFCWALL").len() - 3);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{unset});\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn relation(global_id: &str) -> AssociationDraft<'_> {
    AssociationDraft::new(global_id, &[WALL]).name("Classified")
}

/// Author through every writer with the values `version` can hold.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let since = |value| ifc4.then_some(value);
    let mut tx = Transaction::new(model);
    let draft = {
        let mut draft = ClassificationDraft::new("Uniclass")
            .source("NBS")
            .edition("2024");
        draft.edition_date = since("2024-01-01");
        draft.description = since("Unified classification");
        draft.location = since("https://uniclass.thenbs.com");
        draft.reference_tokens = ifc4.then_some(&["_"][..]);
        draft
    };
    let system = create_classification_in(&mut tx, model, draft).expect("system");
    let reference = {
        let mut draft = ClassificationReferenceDraft::new()
            .identification("Pr_20")
            .name("Products")
            .referenced_source(system);
        draft.description = since("Structural");
        draft.sort = since("20");
        draft
    };
    let reference = create_classification_reference(&mut tx, model, reference).expect("ref");
    let document = |identification| {
        let mut draft = DocumentDraft::new(identification, "Spec")
            .revision("B")
            .document_owner(ORGANIZATION)
            .editors(&[PERSON])
            .confidentiality("PUBLIC")
            .status("FINALDRAFT");
        draft.location = since("https://example.org/spec.pdf");
        draft.creation_time = since("2026-09-28T00:00:00");
        draft.electronic_format = since("application/pdf");
        draft.valid_from = since("2026-09-28");
        draft
    };
    let current = create_document(&mut tx, model, document("DOC-1")).expect("document");
    let old = create_document(&mut tx, model, document("DOC-0")).expect("document");
    let document_ref = {
        let mut draft = DocumentReferenceDraft::new().identification("7");
        draft.name = (!ifc4).then_some("Spec sheet");
        draft.description = since("Sheet 7");
        draft.referenced_document = ifc4.then_some(current);
        draft
    };
    let document_ref = create_document_reference(&mut tx, model, document_ref).expect("doc ref");
    let library = {
        let mut draft = LibraryDraft::new("Products")
            .version("3")
            .publisher(ORGANIZATION);
        draft.version_date = since("2026-09-28T00:00:00");
        draft.location = since("https://example.org/lib");
        draft
    };
    let library = create_library(&mut tx, model, library).expect("library");
    let library_ref = {
        let mut draft = LibraryReferenceDraft::new()
            .identification("P1")
            .name("Pump");
        draft.language = since("en");
        draft.referenced_library = ifc4.then_some(library);
        draft
    };
    let library_ref = create_library_reference(&mut tx, model, library_ref).expect("lib ref");
    let superseding = relate_documents(&mut tx, model, current, &[old], Some("SUPERSEDES"))
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
        current,
        old,
        document_ref,
        library,
        library_ref,
        superseding,
    ];
    written.extend(associations);
    if ifc4 {
        let draft = ExternalReferenceRelationshipDraft::new(library_ref, &[ORGANIZATION]);
        written.push(create_external_reference_relationship(&mut tx, model, draft).expect("err"));
    }
    tx.commit(model).expect("commit");
    written
}

/// Error findings `ifc-validate` reports on `ids`, against `version`.
fn errors(model: &Model, version: SchemaVersion, ids: &[EntityId]) -> Vec<String> {
    let report = ifc_validate::validate(model, for_version(version).expect("bundled"));
    report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .filter(|finding| match &finding.path {
            ifc_validate::Path::Entity(id) | ifc_validate::Path::Attribute { entity: id, .. } => {
                ids.contains(id)
            }
            ifc_validate::Path::File => false,
            // A path kind a later ifc-validate adds names no authored record.
            _ => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_classification_records_validate_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    }
}

/// The oracle above is only trusted because it fails when it should: the
/// IFC4 `IfcClassification` layout written into an IFC2X3 model, as the
/// writers did before #194, is an error finding.
#[test]
fn the_validator_catches_an_ifc4_classification_in_ifc2x3() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let text = |s: &str| Value::Text(s.into());
    let classification = tx.create(ifc::Entity::new(
        "IFCCLASSIFICATION",
        vec![
            text("NBS"),
            text("2024"),
            Value::Null,
            text("Uniclass"),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[classification]).is_empty(),
        "a seven-attribute IfcClassification is not IFC2X3"
    );
}

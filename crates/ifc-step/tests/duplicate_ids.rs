//! Two records with one id are reported on every read, not only under
//! `check_references` (#626).

use ifc_model::{Codec, DiagnosticKind, EntityId, Model, ModelError, Value};
use ifc_step::{ParseOptions, StepReader};

fn wrap(records: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{records}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

const TEXT: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
    FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
    DATA;\n#1=IFCSITE('first');\n#2=IFCSITE('mid');\n#1=IFCSITE('second');\nENDSEC;\n\
    END-ISO-10303-21;\n";

/// Every way to read: lazy and eager, with and without `check_references`.
fn readers(base: ParseOptions) -> Vec<(&'static str, StepReader)> {
    vec![
        ("default", StepReader::new(base)),
        ("eager", StepReader::new(base).eager()),
        ("checking", StepReader::new(base.check_references(true))),
    ]
}

fn first_text(model: &Model) -> Value {
    model.get(EntityId(1)).expect("#1").attributes[0].clone()
}

#[test]
fn a_strict_read_refuses_the_id() {
    for (name, reader) in readers(ParseOptions::strict()) {
        let error = reader.read_bytes(TEXT.as_bytes()).expect_err(name);
        assert!(
            matches!(error, ModelError::DuplicateId(EntityId(1))),
            "{name}: {error:?}"
        );
        let error = reader.check(TEXT.as_bytes()).expect_err(name);
        assert!(
            matches!(error, ModelError::DuplicateId(EntityId(1))),
            "{name}"
        );
        let failure = reader.try_read_owned(TEXT.as_bytes().to_vec());
        assert!(failure.is_err(), "{name}");
    }
}

#[test]
fn the_error_places_the_later_record() {
    let error = ifc_step::StepError::DuplicateId {
        id: EntityId(1),
        offset: 7,
    };
    assert_eq!(error.to_string(), "duplicate instance id #1 at byte 7");
}

#[test]
fn a_lenient_read_keeps_the_later_record_and_reports_the_id() {
    let at = TEXT.rfind("#1=").unwrap();
    for (name, reader) in readers(ParseOptions::lenient()) {
        let model = reader.read_bytes(TEXT.as_bytes()).expect(name);
        assert_eq!(model.len(), 2, "{name}");
        assert_eq!(first_text(&model), Value::Text("second".into()), "{name}");
        let found: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| d.kind() == DiagnosticKind::DuplicateId)
            .collect();
        assert_eq!(found.len(), 1, "{name}: {:?}", model.diagnostics());
        let d = found[0];
        assert_eq!(d.instance(), Some(EntityId(1)), "{name}");
        assert_eq!(d.byte_range().expect("range").start, at, "{name}");
        assert_eq!(d.position().expect("position").line(), 10, "{name}");
    }
}

#[test]
fn the_report_is_the_same_with_and_without_check_references() {
    let mixed = wrap("#1=IFCSITE('a');\n#1=IFCSITE('c');\n#2=IFCSITE(1E-05);");
    for text in [TEXT, mixed.as_str()] {
        let plain = StepReader::new(ParseOptions::lenient())
            .read_bytes(text.as_bytes())
            .unwrap();
        let checked = StepReader::new(ParseOptions::lenient().check_references(true))
            .read_bytes(text.as_bytes())
            .unwrap();
        assert_eq!(plain.diagnostics(), checked.diagnostics());
    }
    let kinds: Vec<_> = StepReader::new(ParseOptions::lenient())
        .read_bytes(mixed.as_bytes())
        .unwrap()
        .diagnostics()
        .iter()
        .map(|d| d.kind())
        .collect();
    assert_eq!(
        kinds,
        [
            DiagnosticKind::DuplicateId,
            DiagnosticKind::RealWithoutPoint
        ]
    );
}

fn duplicates(text: &str) -> Vec<ifc_model::Diagnostic> {
    StepReader::new(ParseOptions::lenient())
        .read_bytes(text.as_bytes())
        .unwrap()
        .diagnostics()
        .iter()
        .filter(|d| d.kind() == DiagnosticKind::DuplicateId)
        .cloned()
        .collect()
}

#[test]
fn a_skipped_record_before_the_duplicate_does_not_misplace_it() {
    // A: the skipped record shifts the sink's count against the file's.
    let a = wrap("#1=IFCSITE('a');\n#2=IFCSITE(1.2.3);\n#3=IFCSITE('b');\n#1=IFCSITE('c');");
    let found = duplicates(&a);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].instance(), Some(EntityId(1)));
    assert_eq!(
        found[0].byte_range().unwrap().start,
        a.rfind("#1=").unwrap()
    );
    // B: the damaged record's id is unreadable.
    let b = wrap("#1=IFCSITE('a');\n#7\nXXSITE('x');\n#1=IFCSITE('c');");
    let found = duplicates(&b);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].instance(), Some(EntityId(1)));
    if let Some(range) = found[0].byte_range() {
        assert_eq!(range.start, b.rfind("#1=").unwrap());
    }
}

#[test]
fn a_skipped_record_of_the_same_id_leaves_the_duplicate_unplaced() {
    let text = wrap("#1=IFCSITE('a');\n#1=IFCSITE(1.2.3);\n#1=IFCSITE('c');");
    let found = duplicates(&text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].instance(), Some(EntityId(1)));
    assert_eq!(found[0].byte_range(), None);
}

#[test]
fn an_index_refuses_the_id_and_locates_it() {
    let error = ifc_step::Index::scan(TEXT.as_bytes())
        .err()
        .expect("refused");
    let ifc_step::StepError::DuplicateId { id, offset } = error else {
        panic!("{error:?}");
    };
    assert_eq!(id, EntityId(1));
    assert_eq!(offset, TEXT.rfind("#1=").unwrap());
    let located = ifc_step::SyntaxLocation::of_step_error(
        TEXT.as_bytes(),
        &ifc_step::StepError::DuplicateId { id, offset },
    )
    .expect("located");
    assert_eq!(located.position().line(), 10);
}

#[test]
fn a_file_without_duplicates_reads_as_before() {
    let text = TEXT.replace("#1=IFCSITE('second')", "#3=IFCSITE('second')");
    for base in [ParseOptions::strict(), ParseOptions::lenient()] {
        for (name, reader) in readers(base) {
            let model = reader.read_bytes(text.as_bytes()).expect(name);
            assert_eq!(model.len(), 3, "{name}");
            assert!(model.diagnostics().is_empty(), "{name}");
        }
    }
}

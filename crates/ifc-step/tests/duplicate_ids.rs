//! Two records with one id are reported on every read, not only under
//! `check_references` (#626).

use ifc_model::{Codec, DiagnosticKind, EntityId, Model, ModelError, Value};
use ifc_step::{ParseOptions, StepReader};

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
    let plain = StepReader::new(ParseOptions::lenient())
        .read_bytes(TEXT.as_bytes())
        .unwrap();
    let checked = StepReader::new(ParseOptions::lenient().check_references(true))
        .read_bytes(TEXT.as_bytes())
        .unwrap();
    assert_eq!(plain.diagnostics(), checked.diagnostics());
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

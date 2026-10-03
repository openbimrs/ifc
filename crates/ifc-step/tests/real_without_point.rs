//! REALs written without a decimal point (#285): strict refuses and names
//! the token, lenient reads the REAL and says so, the writer always emits
//! the point.

use ifc_model::{Codec, EntityId, Model, ModelError, Value};
use ifc_step::{Index, ParseOptions, StepCodec, StepError, StepReader};

fn exchange(records: &str) -> String {
    format!(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\n\
         FILE_SCHEMA(('IFC4X3_ADD2'));\n\
         ENDSEC;\n\
         DATA;\n\
         {records}\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n"
    )
}

/// One point per token, so each test isolates one spelling.
fn point(token: &str) -> String {
    exchange(&format!(
        "#1= IFCCARTESIANPOINT((0.,{token}));\n#2= IFCPERSON($,$,'1E2',$,$,$,$,$);"
    ))
}

const TOKENS: [(&str, f64); 3] = [("1E-05", 1e-5), ("-2E3", -2e3), ("3e+2", 3e2)];

fn strict_readers() -> [(&'static str, StepReader); 2] {
    [
        ("lazy", StepReader::new(ParseOptions::strict())),
        ("eager", StepReader::new(ParseOptions::strict()).eager()),
    ]
}

fn coordinates(model: &Model) -> Vec<Value> {
    let entity = model.get(EntityId(1)).expect("#1 is read");
    match &entity.attributes[0] {
        Value::List(items) => items.clone(),
        other => panic!("coordinates are a list, got {other:?}"),
    }
}

#[test]
fn strict_reading_refuses_the_token_and_names_it() {
    for (token, _) in TOKENS {
        let file = point(token);
        let offset = file.find(token).expect("token in file");
        for (mode, reader) in strict_readers() {
            let error = reader
                .read_bytes(file.as_bytes())
                .expect_err("ISO 10303-21 REAL requires the point");
            match &error {
                ModelError::Syntax { offset: at, detail } => {
                    assert_eq!(*at, offset, "{mode} {token}: {error}");
                    assert!(detail.contains(&format!("`{token}`")), "{mode}: {detail}");
                    assert!(detail.contains("decimal point"), "{mode}: {detail}");
                }
                other => panic!("{mode} {token}: expected a syntax error, got {other:?}"),
            }
        }
        // `StepCodec` is the strict reader by default.
        assert!(StepCodec.read_bytes(file.as_bytes()).is_err());
    }
}

#[test]
fn the_index_decode_names_the_token_too() {
    let file = point("1E-05");
    let index = Index::scan(file.as_bytes()).expect("framing does not decode values");
    match index.entity(EntityId(1)) {
        Err(StepError::RealWithoutDecimalPoint { offset, token }) => {
            assert_eq!(token, "1E-05");
            assert_eq!(offset, file.find("1E-05").unwrap());
        }
        other => panic!("expected RealWithoutDecimalPoint, got {other:?}"),
    }
}

#[test]
fn lenient_reading_accepts_the_real_and_reports_it() {
    for (token, value) in TOKENS {
        let file = point(token);
        let model = StepCodec::lenient()
            .read_bytes(file.as_bytes())
            .unwrap_or_else(|error| panic!("{token}: {error}"));
        assert_eq!(model.len(), 2, "{token}: nothing is skipped");
        assert_eq!(
            coordinates(&model),
            [Value::Real(0.0), Value::Real(value)],
            "{token}"
        );
        // Text that merely looks like the token is untouched.
        assert_eq!(
            model.get(EntityId(2)).unwrap().attributes[2],
            Value::Text("1E2".into())
        );
        assert_eq!(model.diagnostics().len(), 1, "{token}");
        let diagnostic = &model.diagnostics()[0];
        assert!(
            diagnostic.detail().contains(&format!("`{token}`")),
            "{}",
            diagnostic.detail()
        );
        let range = diagnostic.byte_range().expect("located");
        assert_eq!(&file[range.clone()], token);
    }
}

#[test]
fn lenient_offsets_of_other_diagnostics_refer_to_the_original_bytes() {
    // A repaired token before a damaged record: the skipped record's range
    // must still quote the original source, not the repaired copy.
    let file = exchange(
        "#1= IFCCARTESIANPOINT((1E2,2E2));\n\
         #2\n\
         IFCBROKEN(;\n\
         #3= IFCORGANIZATION($,'o',$,$,$);",
    );
    let model = StepCodec::lenient().read_bytes(file.as_bytes()).unwrap();
    assert_eq!(model.len(), 2);
    let skipped = model
        .diagnostics()
        .iter()
        .find(|d| d.detail().contains("skipped malformed data record"))
        .expect("the damaged record is reported");
    let range = skipped.byte_range().unwrap().clone();
    assert!(file[range].starts_with("#2"), "{skipped}");
    assert_eq!(model.diagnostics().len(), 3);
}

#[test]
fn a_token_that_is_not_a_number_is_still_refused() {
    // `1E` has no exponent digits; `1E-` neither. Neither is a REAL, so
    // lenient reading skips the record instead of inventing a value.
    for token in ["1E", "1E-", "1EE2"] {
        let file = point(token);
        for (mode, reader) in strict_readers() {
            assert!(
                matches!(
                    reader.read_bytes(file.as_bytes()),
                    Err(ModelError::Syntax { .. })
                ),
                "{mode} {token}"
            );
        }
        let model = StepCodec::lenient().read_bytes(file.as_bytes()).unwrap();
        assert!(model.get(EntityId(1)).is_none(), "{token}");
        assert_eq!(model.diagnostics().len(), 1, "{token}");
        assert!(
            model.diagnostics()[0]
                .detail()
                .contains("skipped malformed data record"),
            "{token}: {}",
            model.diagnostics()[0].detail()
        );
    }
}

#[test]
fn a_conforming_file_is_untouched_by_lenient_reading() {
    let file = point("1.E-05");
    let model = StepCodec::lenient().read_bytes(file.as_bytes()).unwrap();
    assert!(model.is_complete());
    assert_eq!(coordinates(&model), [Value::Real(0.0), Value::Real(1e-5)]);
}

#[test]
fn the_writer_emits_the_point_and_the_value_round_trips() {
    for (token, value) in TOKENS {
        let model = StepCodec::lenient()
            .read_bytes(point(token).as_bytes())
            .unwrap();
        let mut out = Vec::new();
        StepCodec.write(&model, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            ifc_step_written_reals(&text)
                .iter()
                .all(|real| real.contains('.')),
            "{token}: {text}"
        );
        let again = StepCodec
            .read_bytes(text.as_bytes())
            .expect("strict re-read");
        assert!(again.is_complete());
        assert_eq!(coordinates(&again), [Value::Real(0.0), Value::Real(value)]);
    }
}

/// The numbers inside `#1`'s coordinate list as written.
fn ifc_step_written_reals(text: &str) -> Vec<String> {
    let line = text
        .lines()
        .find(|line| line.starts_with("#1="))
        .expect("#1 is written");
    let list = &line[line.find("((").unwrap() + 2..line.find("))").unwrap()];
    list.split(',').map(str::to_owned).collect()
}

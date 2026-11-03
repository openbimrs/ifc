//! `try_read_owned`, `SyntaxLocation` and `check` (#616).

use ifc_model::{Codec, ModelError};
use ifc_step::{ParseOptions, StepCodec, StepReader, SyntaxLocation};
use openbim_step::{Source, Span};

fn exchange(records: &str) -> Vec<u8> {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{records}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
    .into_bytes()
}

/// The defect in AC20-FZK-Haus.ifc, as in `recovery.rs`.
fn ac20() -> Vec<u8> {
    exchange(
        "#79106= IFCCONNECTIONSURFACEGEOMETRY(#79104,$);\n\
         #7\n\
         ACEBOUNDARY('13UjdmCIGNmNY28Gtm7OlY',#12,'2ndLevel','2a',#76214,#67536,#79106,.PHYSICAL.,.EXTERNAL.);\n\
         #79110= IFCORGANIZATION($,'o',$,$,$);",
    )
}

fn line_of(bytes: &[u8], needle: &str) -> usize {
    let text = String::from_utf8_lossy(bytes);
    1 + text[..text.find(needle).unwrap()].matches('\n').count()
}

#[test]
fn a_failed_read_returns_the_buffer_and_locates_the_error() {
    let strict_eager = StepReader::new(ParseOptions::strict()).eager();
    for (name, (pointer, result)) in [
        ("lazy", {
            let bytes = ac20();
            let pointer = bytes.as_ptr();
            (pointer, StepCodec.try_read_owned(bytes))
        }),
        ("eager", {
            let bytes = ac20();
            let pointer = bytes.as_ptr();
            (pointer, strict_eager.try_read_owned(bytes))
        }),
    ] {
        let failure = result.expect_err("strict refuses AC20");
        assert_eq!(failure.bytes.as_ptr(), pointer, "{name}: buffer copied");
        assert_eq!(failure.bytes, ac20(), "{name}");
        let location = failure.location().expect("a syntax error");
        let ModelError::Syntax { offset, .. } = failure.error else {
            panic!("{name}: {:?}", failure.error);
        };
        assert_eq!(location.offset(), offset);
        let reference = Source::new("", &failure.bytes).location(Span::new(offset, offset));
        assert_eq!(location.position().line(), reference.line, "{name}");
        assert_eq!(location.position().column(), reference.column, "{name}");
        assert_eq!(
            location.position().line(),
            line_of(&failure.bytes, "ACEBOUNDARY"),
            "{name}"
        );
        assert!(location.excerpt().contains("ACEBOUNDARY("), "{name}");
        let offending = failure.bytes[offset] as char;
        assert_eq!(
            location
                .excerpt()
                .chars()
                .nth(location.excerpt_column() - 1),
            Some(offending),
            "{name}"
        );
        assert!(location.to_string().starts_with(&format!(
            "line {}, column {}: ",
            location.position().line(),
            location.position().column()
        )));
        // The returned buffer reads under the lenient policy.
        let model = StepCodec::lenient()
            .read_owned(failure.bytes)
            .expect("recovery reads AC20");
        assert_eq!(model.len(), 2);
    }
}

#[test]
fn a_long_line_is_quoted_as_a_window_around_the_offset() {
    let filler = "A".repeat(1000);
    let mut input = format!("{filler}!{filler}").into_bytes();
    input.extend_from_slice(b"\nnext line\n");
    let at = SyntaxLocation::at(&input, 1000);
    assert_eq!(at.position().column(), 1001);
    assert!(at.excerpt().chars().count() <= 122);
    assert!(at.excerpt().starts_with('…') && at.excerpt().ends_with('…'));
    assert_eq!(at.excerpt().chars().nth(at.excerpt_column() - 1), Some('!'));
    // Near the start only the right end is cut.
    let start = SyntaxLocation::at(&input, 3);
    assert!(!start.excerpt().starts_with('…') && start.excerpt().ends_with('…'));
    assert_eq!(start.excerpt_column(), 4);
    // Past the end clamps.
    assert_eq!(SyntaxLocation::at(b"ab", 99).offset(), 2);
}

#[test]
fn only_syntax_errors_are_located() {
    assert!(SyntaxLocation::locate(b"", &ModelError::Io("x".into())).is_none());
    let wrong = StepCodec.try_read_owned(b"not step".to_vec()).unwrap_err();
    assert!(matches!(wrong.error, ModelError::WrongFormat { .. }));
    assert_eq!(wrong.bytes, b"not step");
    assert!(wrong.location().is_none());
}

#[test]
fn check_agrees_with_read_bytes() {
    let clean = exchange("#1=IFCSITE($);\n#2=IFCSITE(#1);");
    let missing_endsec = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
        FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
        DATA;\n#1=IFCSITE($);\n"
        .to_vec();
    let broken_header =
        b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n"
            .to_vec();
    let unterminated = exchange("#1=IFCSITE('never closed);");
    let complex = exchange("#1=(IFCSITE()IFCROOT());\n#2=IFCSITE($);");
    let real = exchange("#1=IFCCARTESIANPOINT((1E-05,0.));");
    let dangling = exchange("#1=IFCSITE(#9);\n#1=IFCSITE($);");
    let cases = [
        ("clean", clean),
        ("ac20", ac20()),
        ("missing ENDSEC", missing_endsec),
        ("broken header", broken_header),
        ("unterminated string", unterminated),
        ("complex", complex),
        ("1E-05", real),
        ("dangling and duplicate", dangling),
        ("not step", b"hello".to_vec()),
    ];
    let readers = [
        ("strict", StepReader::new(ParseOptions::strict())),
        (
            "strict eager",
            StepReader::new(ParseOptions::strict()).eager(),
        ),
        ("lenient", StepReader::new(ParseOptions::lenient())),
        (
            "lenient+refs",
            StepReader::new(ParseOptions::lenient().check_references(true)),
        ),
    ];
    for (case, bytes) in &cases {
        for (policy, reader) in &readers {
            assert_eq!(
                format!("{:?}", reader.check(bytes)),
                format!("{:?}", reader.read_bytes(bytes).map(drop)),
                "{case} under {policy}"
            );
        }
    }
}

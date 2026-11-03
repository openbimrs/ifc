//! What a read's diagnostics say: kind, instance, referrer, position, quote.

use ifc_model::Codec;
use ifc_model::{Diagnostic, DiagnosticKind, EntityId, SourcePosition};
use ifc_step::{ParseOptions, StepReader};

fn exchange(records: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{records}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

/// The defect in AC20-FZK-Haus.ifc, as in `recovery.rs`.
fn damaged() -> String {
    exchange(
        "#79106= IFCCONNECTIONSURFACEGEOMETRY(#79104,$);\n\
         #7\n\
         ACEBOUNDARY('13UjdmCIGNmNY28Gtm7OlY',#12,'2ndLevel','2a',#76214,#67536,#79106,.PHYSICAL.,.EXTERNAL.);\n\
         #79110= IFCORGANIZATION($,'o',$,$,$);",
    )
}

fn read(codec: &StepReader, text: &str) -> Vec<Diagnostic> {
    codec
        .read_bytes(text.as_bytes())
        .expect("reads")
        .diagnostics()
        .to_vec()
}

fn line_of(text: &str, needle: &str) -> usize {
    let at = text.find(needle).expect("needle");
    1 + text[..at].matches('\n').count()
}

fn lenient() -> StepReader {
    StepReader::new(ParseOptions::lenient())
}

fn lenient_checking() -> StepReader {
    StepReader::new(ParseOptions::lenient().check_references(true))
}

#[test]
fn a_skipped_record_is_located_and_quoted() {
    let text = damaged();
    let found = read(&lenient(), &text);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(
        d.byte_range().expect("range").start,
        text.find("#7\n").unwrap()
    );
    assert_eq!(
        d.position(),
        Some(SourcePosition::new(line_of(&text, "#7\n"), 1))
    );
    assert!(
        d.excerpt().unwrap().starts_with("#7 ACEBOUNDARY("),
        "{:?}",
        d.excerpt()
    );
    assert_eq!(d.instance(), Some(EntityId(7)));
    assert_eq!(d.referrer(), None);
}

#[test]
fn a_skipped_record_after_a_stray_run_names_the_id_it_starts_with() {
    let text = exchange("#17\n XXSITE('a');\n#18=IFCSITE(#17);");
    let found = read(&lenient(), &text);
    assert_eq!(found[0].kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(found[0].instance(), Some(EntityId(17)));

    let found = read(&lenient_checking(), &text);
    let dangling: Vec<_> = found
        .iter()
        .filter(|d| d.kind() == DiagnosticKind::DanglingReference)
        .collect();
    assert_eq!(dangling.len(), 1, "{found:?}");
    assert_eq!(dangling[0].instance(), Some(EntityId(17)));
    assert_eq!(dangling[0].referrer(), Some(EntityId(18)));
}

#[test]
fn a_run_without_an_id_or_with_an_overflowing_one_names_no_instance() {
    let found = read(&lenient(), &exchange("XX(;\n#1=IFCSITE($);"));
    assert_eq!(found[0].kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(found[0].instance(), None);
    let found = read(
        &lenient(),
        &exchange("#99999999999999999999999\n X(;\n#1=IFCSITE($);"),
    );
    assert_eq!(found[0].kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(found[0].instance(), None);
}

#[test]
fn a_dangling_reference_names_the_missing_id_and_the_holder() {
    let text = exchange("#16=IFCSITE($);\n#18=IFCLOCALPLACEMENT(#17,$);\n#23=IFCSITE(#17);");
    let found = read(&lenient_checking(), &text);
    assert_eq!(found.len(), 2, "{found:?}");
    for (d, holder) in found.iter().zip([18, 23]) {
        assert_eq!(d.kind(), DiagnosticKind::DanglingReference);
        assert_eq!(d.instance(), Some(EntityId(17)));
        assert_eq!(d.referrer(), Some(EntityId(holder)));
        let needle = format!("#{holder}=");
        assert_eq!(d.position().unwrap().line(), line_of(&text, &needle));
        assert_eq!(d.position().unwrap().column(), 1);
    }
}

#[test]
fn a_duplicate_id_names_the_id() {
    let text = exchange("#1=IFCSITE($);\n#1=IFCSITE($);");
    let found = read(&lenient_checking(), &text);
    let d = found
        .iter()
        .find(|d| d.kind() == DiagnosticKind::DuplicateId)
        .expect("duplicate");
    assert_eq!(d.instance(), Some(EntityId(1)));
    assert_eq!(d.referrer(), None);
    assert_eq!(
        d.position().unwrap().line(),
        line_of(&text, "#1=IFCSITE($);\n#1=") + 1
    );
}

#[test]
fn a_real_without_a_point_quotes_the_token_and_names_no_instance() {
    let text = exchange("#1=IFCCARTESIANPOINT((1E-05,0.));");
    let found = read(&lenient(), &text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind(), DiagnosticKind::RealWithoutPoint);
    assert_eq!(found[0].excerpt(), Some("1E-05"));
    assert_eq!(found[0].instance(), None);
    assert_eq!(found[0].referrer(), None);
}

#[test]
fn a_complex_instance_is_skipped_with_its_id_and_no_range() {
    let text = exchange("#5=(IFCSITE()IFCROOT());\n#6=IFCSITE($);");
    let found = read(&lenient(), &text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(found[0].instance(), Some(EntityId(5)));
    assert_eq!(found[0].byte_range(), None);
}

#[test]
fn a_skipped_record_does_not_take_the_id_of_a_record_that_was_read() {
    let text = exchange("#45=IFCSITE($);\n#10=IFCSITE(#1;#45,#46);\n#11=IFCSITE($);");
    let found = read(&lenient(), &text);
    let skipped: Vec<_> = found
        .iter()
        .filter(|d| d.kind() == DiagnosticKind::SkippedRecord)
        .collect();
    // `#10=IFCSITE(#1;` is cut at the `;`; the rest, `#45,#46);`, starts with
    // an id the model holds, which must not be named as the skipped one.
    let instances: Vec<_> = skipped.iter().map(|d| d.instance()).collect();
    assert_eq!(instances, [Some(EntityId(10)), None], "{found:?}");
}

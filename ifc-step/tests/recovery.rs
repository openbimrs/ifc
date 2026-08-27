//! Reading a damaged export: strict by default, opt-in recovery with diagnostics.

use ifc_model::{Codec, Model};
use ifc_step::{OnMalformed, StepCodec, StepReader};

/// The exact defect in AC20-FZK-Haus.ifc: a truncated write that swallowed the
/// middle of one record, leaving `#7` glued to the tail of the next entity.
fn damaged() -> String {
    exchange(
        "#79106= IFCCONNECTIONSURFACEGEOMETRY(#79104,$);\n\
         #7\n\
         ACEBOUNDARY('13UjdmCIGNmNY28Gtm7OlY',#12,'2ndLevel','2a',#76214,#67536,#79106,.PHYSICAL.,.EXTERNAL.);\n\
         #79110= IFCORGANIZATION($,'o',$,$,$);",
    )
}

fn exchange(records: &str) -> String {
    format!(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\n\
         FILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\n\
         DATA;\n\
         {records}\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n"
    )
}

fn type_names(model: &Model) -> Vec<String> {
    model
        .iter()
        .map(|(_, entity)| entity.type_name.to_string())
        .collect()
}

#[test]
fn the_default_codec_rejects_a_damaged_file() {
    let error = StepCodec
        .read_bytes(damaged().as_bytes())
        .expect_err("strict reading must not silently drop a record");
    assert!(error.to_string().contains("syntax error"), "{error}");
    assert!(StepReader::default()
        .read_bytes(damaged().as_bytes())
        .is_err());
}

#[test]
fn the_lenient_codec_loads_the_rest_and_reports_what_it_dropped() {
    let model = StepCodec::lenient()
        .read_bytes(damaged().as_bytes())
        .expect("recovery must load the readable records");

    assert_eq!(
        type_names(&model),
        ["IFCCONNECTIONSURFACEGEOMETRY", "IFCORGANIZATION"]
    );
    assert_eq!(model.header().schema, ["IFC4"]);

    assert!(!model.is_complete());
    assert_eq!(model.diagnostics().len(), 1);
    let diagnostic = &model.diagnostics()[0];
    assert!(
        diagnostic
            .detail()
            .contains("skipped malformed data record"),
        "{}",
        diagnostic.detail()
    );

    // The reported range must quote the dropped source bytes exactly.
    let source = damaged();
    let range = diagnostic.byte_range().expect("STEP reports byte offsets");
    let dropped = &source.as_bytes()[range.clone()];
    assert!(String::from_utf8_lossy(dropped).contains("ACEBOUNDARY"));
}

#[test]
fn a_clean_file_is_complete_under_both_policies() {
    let clean = exchange("#1= IFCPERSON($,$,'a',$,$,$,$,$);\n#2= IFCORGANIZATION($,'o',$,$,$);");
    let strict = StepCodec.read_bytes(clean.as_bytes()).expect("clean file");
    let lenient = StepCodec::lenient()
        .read_bytes(clean.as_bytes())
        .expect("clean file");
    for model in [strict, lenient] {
        assert_eq!(model.len(), 2);
        assert!(model.is_complete());
        assert!(model.diagnostics().is_empty());
    }
}

#[test]
fn recovery_reports_records_that_are_valid_step_but_unrepresentable() {
    // A complex instance is well-formed STEP the IFC record model cannot hold.
    let complex = exchange("#1= (IFCPERSON($,$,'a',$,$,$,$,$) IFCORGANIZATION($,'o',$,$,$));\n#2= IFCPERSON($,$,'b',$,$,$,$,$);");

    assert!(StepCodec.read_bytes(complex.as_bytes()).is_err());

    let model = StepCodec::lenient()
        .read_bytes(complex.as_bytes())
        .expect("recovery");
    assert_eq!(type_names(&model), ["IFCPERSON"]);
    assert_eq!(model.diagnostics().len(), 1);
    assert!(
        model.diagnostics()[0].detail().contains("#1"),
        "{}",
        model.diagnostics()[0].detail()
    );
}

#[test]
fn header_and_format_defects_stay_fatal_when_recovering() {
    let no_marker = "#1= IFCPERSON($,$,'a',$,$,$,$,$);\n";
    assert!(StepCodec::lenient()
        .read_bytes(no_marker.as_bytes())
        .is_err());

    let missing_schema = "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\n\
         ENDSEC;\n\
         DATA;\n\
         #1= IFCPERSON($,$,'a',$,$,$,$,$);\n\
         ENDSEC;\n\
         END-ISO-10303-21;\n";
    assert!(StepCodec::lenient()
        .read_bytes(missing_schema.as_bytes())
        .is_err());
}

#[test]
fn the_policy_is_selectable_explicitly() {
    let source = damaged();
    assert!(StepReader::default()
        .on_malformed_record(OnMalformed::Skip)
        .read_bytes(source.as_bytes())
        .is_ok());
    assert!(StepCodec::lenient()
        .on_malformed_record(OnMalformed::Abort)
        .read_bytes(source.as_bytes())
        .is_err());
}

#[test]
fn a_recovered_model_still_round_trips_what_it_kept() {
    let codec = StepCodec::lenient();
    let model = codec.read_bytes(damaged().as_bytes()).expect("recovery");
    let written = codec.write_bytes(&model).expect("write");

    // Re-reading the export is clean: the damage is gone, not carried forward.
    let reparsed = StepCodec
        .read_bytes(&written)
        .expect("a recovered model must export valid STEP");
    assert_eq!(type_names(&reparsed), type_names(&model));
    assert!(reparsed.is_complete());
}

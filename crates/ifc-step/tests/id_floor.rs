//! After a read, `next_id` is above every id the file declared or referenced
//! (#617), for strict and lenient, lazy and eager reads alike.

use ifc_model::{Codec, Entity, EntityId, History, Model};
use ifc_step::{ParseOptions, StepCodec, StepReader};

fn exchange(records: &str) -> Vec<u8> {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('n','t',(''),(''),'p','o','a');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{records}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
    .into_bytes()
}

fn lenient() -> StepReader {
    StepReader::new(ParseOptions::lenient())
}

#[test]
fn a_skipped_record_keeps_its_id_taken() {
    let bytes = exchange("#1=IFCSITE($);\n#400\n XX(;\n#2=IFCSITE($);");
    let model = lenient().read_bytes(&bytes).unwrap();
    assert_eq!(model.next_id(), EntityId(401));
}

#[test]
fn a_malformed_record_above_everything_keeps_its_id_taken() {
    let bytes = exchange("#1=IFCSITE($);\n#700\nX(\n#2=IFCSITE($);");
    let model = lenient().read_bytes(&bytes).unwrap();
    assert_eq!(model.next_id(), EntityId(701));
}

#[test]
fn an_unrepresentable_record_and_a_reference_raise_the_floor() {
    let bytes = exchange("#1=IFCSITE($);\n#500=(IFCSITE()IFCROOT());\n#10=IFCSITE(#600);");
    let model = lenient().read_bytes(&bytes).unwrap();
    assert_eq!(model.next_id(), EntityId(601));
    let only_complex = exchange("#1=IFCSITE($);\n#500=(IFCSITE()IFCROOT());");
    let model = lenient().read_bytes(&only_complex).unwrap();
    assert_eq!(model.next_id(), EntityId(501));
}

#[test]
fn a_dangling_reference_is_never_handed_out_by_a_strict_read() {
    let bytes = exchange("#1=IFCSITE($);\n#2=IFCLOCALPLACEMENT(#900,$);");
    let lazy = StepCodec.read_bytes(&bytes).unwrap();
    let eager = StepReader::new(ParseOptions::strict())
        .eager()
        .read_bytes(&bytes)
        .unwrap();
    let owned = StepCodec.read_owned(bytes).unwrap();
    for model in [&lazy, &eager, &owned] {
        assert_eq!(model.next_id(), EntityId(901));
    }
}

#[test]
fn undo_of_a_push_after_the_read_returns_to_the_floor() {
    let bytes = exchange("#1=IFCSITE(#900);");
    let mut model = StepCodec.read_bytes(&bytes).unwrap();
    let mut history = History::new(4);
    let id = history.track(&mut model, |m: &mut Model| {
        m.push(Entity::new("IFCWALL", vec![]))
    });
    assert_eq!(id, EntityId(901));
    history.undo(&mut model).unwrap();
    assert_eq!(model.next_id(), EntityId(901));
}

#[test]
fn a_clean_file_still_continues_after_its_highest_id() {
    let bytes = exchange("#1=IFCSITE($);\n#5=IFCSITE(#1);");
    let model = StepCodec.read_bytes(&bytes).unwrap();
    assert_eq!(model.next_id(), EntityId(6));
}

#[test]
fn the_highest_u64_is_no_id() {
    let bytes = exchange("#1=IFCSITE(#18446744073709551615);");
    for result in [
        StepCodec.read_bytes(&bytes).map(drop),
        StepReader::new(ParseOptions::strict())
            .eager()
            .read_bytes(&bytes)
            .map(drop),
    ] {
        let Err(ifc_model::ModelError::Syntax { detail, .. }) = result else {
            panic!("a reference to u64::MAX was accepted");
        };
        assert!(detail.contains("reference id exceeds"), "{detail}");
    }
    let declared = exchange("#18446744073709551615=IFCSITE($);");
    assert!(StepCodec.read_bytes(&declared).is_err());
}

#[test]
fn a_lenient_read_never_reserves_the_highest_u64() {
    for records in [
        "#1=IFCSITE($);\n#18446744073709551615\n X(;",
        "#1=IFCSITE($);\n#18446744073709551615=(IFCSITE()IFCROOT());",
        "#1=IFCSITE($);\n#18446744073709551615=IFCSITE(#18446744073709551615);",
    ] {
        let model = lenient().read_bytes(&exchange(records)).unwrap();
        assert_eq!(model.next_id(), EntityId(2), "{records}");
        assert!(model.diagnostics().iter().all(|d| d.instance().is_none()));
    }
    // The largest id there is leaves exactly one to hand out.
    let model = lenient()
        .read_bytes(&exchange("#18446744073709551614=IFCSITE($);"))
        .unwrap();
    assert_eq!(model.next_id(), EntityId(u64::MAX));
}

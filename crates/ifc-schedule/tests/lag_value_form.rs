//! `IfcLagTime.LagValue` carries the member of its SELECT (#201).
//!
//! IFC4 and IFC4X3 declare `LagValue : IfcTimeOrRatioSelect` with
//! `IfcTimeOrRatioSelect = SELECT (IfcDuration, IfcRatioMeasure)`; IFC2X3
//! declares no `IfcLagTime`. A SELECT value is written as the typed
//! parameter of the member it is, so a duration is `IFCDURATION('P5D')` and
//! a ratio `IFCRATIOMEASURE(0.5)`. Each release is authored, written to STEP,
//! read back with `ifc-step` and read through `sequences`; the STEP text is
//! asserted directly.

use ifc_model::codec::Codec;
use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schedule::{create_lag_time, create_sequence, create_task, sequences, TaskDraft};
use ifc_step::StepCodec;

fn task(tx: &mut Transaction, guid: &str) -> EntityId {
    create_task(
        tx,
        TaskDraft {
            global_id: guid,
            name: Some("task"),
            ..TaskDraft::default()
        },
    )
    .expect("authored task")
}

fn typed(member: &str, value: Value) -> Value {
    Value::Typed {
        type_name: member.into(),
        value: Box::new(value),
    }
}

/// The STEP record of `id`, from the written text.
fn record(text: &str, id: EntityId) -> String {
    let prefix = format!("#{}=", id.0);
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no record {prefix} in\n{text}"))
        .to_owned()
}

/// A lag value as authored, its STEP form, and what reads back.
struct Case {
    authored: Value,
    step: &'static str,
    duration: Option<&'static str>,
    ratio: Option<f64>,
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            authored: Value::Text("P5D".into()),
            step: "IFCDURATION('P5D')",
            duration: Some("P5D"),
            ratio: None,
        },
        Case {
            authored: Value::Real(0.5),
            step: "IFCRATIOMEASURE(0.5)",
            duration: None,
            ratio: Some(0.5),
        },
        Case {
            authored: Value::Integer(2),
            step: "IFCRATIOMEASURE(2.",
            duration: None,
            ratio: Some(2.0),
        },
        Case {
            authored: typed("IFCDURATION", Value::Text("PT4H".into())),
            step: "IFCDURATION('PT4H')",
            duration: Some("PT4H"),
            ratio: None,
        },
        Case {
            authored: typed("IfcRatioMeasure", Value::Real(-0.25)),
            step: "IFCRATIOMEASURE(-0.25)",
            duration: None,
            ratio: Some(-0.25),
        },
    ]
}

#[test]
fn a_lag_value_is_its_select_member_in_every_release() {
    for token in ["IFC4", "IFC4X3_ADD2"] {
        for case in cases() {
            let mut model = Model::default();
            model.header_mut().schema = vec![token.to_owned()];
            let mut tx = Transaction::new(&model);
            let a = task(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu");
            let b = task(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu");
            let lag = create_lag_time(&mut tx, Some("lag"), case.authored.clone(), "WORKTIME")
                .expect("authored lag");
            create_sequence(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", a, b, None, Some(lag))
                .expect("authored sequence");
            tx.commit(&mut model).expect("commit");

            let bytes = StepCodec.write_bytes(&model).expect("written");
            let text = String::from_utf8(bytes.clone()).expect("utf8");
            let line = record(&text, lag);
            assert!(
                line.contains(&format!("'lag',$,$,{}", case.step)),
                "{token} {:?}: {line}",
                case.authored
            );

            let back = StepCodec.read_bytes(&bytes).expect("read back");
            let found = sequences(&back);
            let read = found[0].lag.as_ref().expect("lag");
            assert_eq!(read.duration.as_deref(), case.duration, "{token}");
            assert_eq!(read.ratio, case.ratio, "{token}");
        }
    }
}

#[test]
fn a_value_that_is_no_member_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    for value in [
        typed("IFCDURATION", Value::Real(0.5)),
        typed("IFCRATIOMEASURE", Value::Text("P5D".into())),
        typed("IFCLENGTHMEASURE", Value::Real(0.5)),
        Value::Bool(true),
    ] {
        assert!(
            create_lag_time(&mut tx, None, value.clone(), "WORKTIME").is_err(),
            "{value:?}"
        );
    }
    assert!(tx.is_empty(), "a refusal stages nothing");
}

/// The reader accepts the typed form written now and the bare form written
/// before #201 (and by other tools).
#[test]
fn the_reader_accepts_both_forms() {
    for (lag, duration, ratio) in [
        ("'P5D'", Some("P5D"), None),
        ("IFCDURATION('P5D')", Some("P5D"), None),
        ("0.5", None, Some(0.5)),
        ("IFCRATIOMEASURE(0.5)", None, Some(0.5)),
    ] {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
             #1=IFCLAGTIME('lag',$,$,{lag},.WORKTIME.);\n\
             #2=IFCRELSEQUENCE('2aBcDeFgHiJkLmNoPqRsTu',$,$,$,#3,#4,#1,.FINISH_START.,$);\n\
             ENDSEC;\nEND-ISO-10303-21;\n"
        );
        let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
        let found = sequences(&model);
        let read = found[0].lag.as_ref().expect("lag");
        assert_eq!(read.duration.as_deref(), duration, "{lag}");
        assert_eq!(read.ratio, ratio, "{lag}");
    }
}

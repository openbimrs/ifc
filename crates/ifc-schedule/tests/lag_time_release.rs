//! `create_lag_time_in` binds the model's declared release (#211).
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 declare `IfcLagTime`; IFC2X3 TC1 does not,
//! and carries a sequence's lag as `IfcRelSequence.TimeLag : IfcTimeMeasure`
//! instead. The IFC2X3 call is refused with a typed error and stages
//! nothing; the IFC4 and IFC4X3 calls stage exactly what the release-unaware
//! `create_lag_time` stages.

use ifc_model::{Model, Transaction, Value};
use ifc_schedule::error::ScheduleAuthoringError;
use ifc_schedule::{create_lag_time, create_lag_time_in, SchemaVersion};

fn model(schema: &str) -> Model {
    let mut model = Model::default();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

#[test]
fn ifc2x3_has_no_lag_time_and_nothing_is_staged() {
    let model = model("IFC2X3");
    let mut tx = Transaction::new(&model);
    let refused = create_lag_time_in(
        &mut tx,
        &model,
        Some("cure"),
        Value::Text("P5D".into()),
        "WORKTIME",
    );
    assert_eq!(
        refused,
        Err(ScheduleAuthoringError::EntityNotInSchema {
            entity: "IFCLAGTIME",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert!(tx.is_empty(), "a refused lag stages nothing");
}

#[test]
fn ifc4_and_ifc4x3_stage_what_the_unbound_writer_stages() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema);
        for value in [Value::Text("P5D".into()), Value::Real(0.5)] {
            let mut bound = Transaction::new(&model);
            create_lag_time_in(&mut bound, &model, Some("cure"), value.clone(), "WORKTIME")
                .unwrap_or_else(|error| panic!("{schema}: {error}"));
            let mut unbound = Transaction::new(&model);
            create_lag_time(&mut unbound, Some("cure"), value, "WORKTIME").expect("unbound");
            assert_eq!(bound.edits(), unbound.edits(), "{schema}");
        }
    }
}

#[test]
fn value_refusals_still_apply_in_a_bound_release() {
    let model = model("IFC4");
    let mut tx = Transaction::new(&model);
    assert!(create_lag_time_in(&mut tx, &model, None, Value::Text("P5D".into()), "").is_err());
    assert!(tx.is_empty());
}

//! Model mutation primitives, exercised through a full STEP round trip.
//!
//! ifc-model's own test suite covers the in-memory contract; this test
//! proves an edit written with set_attribute survives write_bytes then
//! read_bytes -- the actual open-signs "move a placed sign" scenario from
//! issue #3.

#![cfg(feature = "step")]
//! Requires `step`: the round trip is the point of the test.

use ifc::{Codec, Model, StepCodec};
use ifc_model::{Entity, Value};

fn point(x: f64, y: f64) -> Entity {
    Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(x), Value::Real(y)])],
    )
}

#[test]
fn moving_a_point_via_set_attribute_survives_a_step_round_trip() {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    let id = model.push(point(0.0, 0.0));

    let previous = model.set_attribute(
        id,
        0,
        Value::List(vec![Value::Real(1000.0), Value::Real(500.0)]),
    );
    assert!(previous.is_some(), "id must already name an entity");

    let bytes = StepCodec
        .write_bytes(&model)
        .expect("edited model serializes");
    let reparsed = StepCodec
        .read_bytes(&bytes)
        .expect("edited model parses back");

    let coords = reparsed
        .get(id)
        .unwrap()
        .attribute(0)
        .unwrap()
        .as_list()
        .unwrap();
    assert_eq!(coords[0].as_f64(), Some(1000.0));
    assert_eq!(coords[1].as_f64(), Some(500.0));
}

#[test]
fn retyping_then_round_tripping_reports_the_new_type() {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    let id = model.push(Entity::new("IFCPROXY", vec![Value::Null]));

    model.retype(id, "IFCWALL");

    let bytes = StepCodec.write_bytes(&model).expect("serializes");
    let reparsed = StepCodec.read_bytes(&bytes).expect("parses back");

    assert!(reparsed.get(id).unwrap().is_type("IFCWALL"));
    assert_eq!(reparsed.ids_of_type("IFCPROXY").len(), 0);
    assert_eq!(reparsed.ids_of_type("IFCWALL").len(), 1);
}

//! Deterministic and scalar-safe STEP writing contracts.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;

#[test]
fn repeated_writes_are_identical_and_preserve_model_record_order() {
    let mut model = Model::new();
    model.insert(EntityId(100), Entity::new("FIRST", vec![Value::Integer(1)]));
    model.insert(EntityId(2), Entity::new("SECOND", vec![Value::Integer(2)]));

    let first = StepCodec.write_bytes(&model).unwrap();
    let second = StepCodec.write_bytes(&model).unwrap();
    assert_eq!(first, second);

    let text = String::from_utf8(first).unwrap();
    assert!(text.find("#100=FIRST").unwrap() < text.find("#2=SECOND").unwrap());
}

#[test]
fn finite_real_extremes_and_negative_zero_round_trip_exactly() {
    let values = [
        -0.0,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        -1.234_567_890_123_456_7e-200,
    ];
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("REALS", values.into_iter().map(Value::Real).collect()),
    );

    let bytes = StepCodec
        .write_bytes(&model)
        .expect("finite values are writable");
    let reparsed = StepCodec
        .read_bytes(&bytes)
        .expect("writer emits valid STEP reals");
    let actual = &reparsed.get(EntityId(1)).unwrap().attributes;
    for (index, expected) in values.iter().enumerate() {
        let Value::Real(found) = actual[index] else {
            panic!("slot {index} did not remain a real");
        };
        assert_eq!(
            found.to_bits(),
            expected.to_bits(),
            "slot {index} changed floating-point value"
        );
    }
}

#[test]
fn non_finite_reals_are_refused_with_entity_and_slot_context() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut model = Model::new();
        model.insert(
            EntityId(42),
            Entity::new("BROKEN", vec![Value::Real(value)]),
        );

        let error = StepCodec
            .write_bytes(&model)
            .expect_err("Part 21 has no non-finite REAL token");
        let message = error.to_string();
        assert!(message.contains("#42"), "{message}");
        assert!(message.contains("slot 0"), "{message}");
        assert!(message.contains("non-finite"), "{message}");
    }
}

#[test]
fn text_quotes_unicode_binary_and_nested_typed_values_round_trip() {
    let expected = Entity::new(
        "SCALARS",
        vec![
            Value::Text("O'Brien – 壁".into()),
            Value::Binary("01001101".into()),
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Value::List(vec![Value::Real(-0.0), Value::Integer(7)])),
            },
        ],
    );
    let mut model = Model::new();
    model.insert(EntityId(9), expected.clone());

    let bytes = StepCodec.write_bytes(&model).unwrap();
    let reparsed = StepCodec.read_bytes(&bytes).unwrap();
    let actual = reparsed.get(EntityId(9)).unwrap();
    assert_eq!(actual.type_name, expected.type_name);
    assert_eq!(actual.attributes[0], expected.attributes[0]);
    assert_eq!(actual.attributes[1], expected.attributes[1]);
    let Value::Typed { value, .. } = &actual.attributes[2] else {
        panic!("typed value was not preserved");
    };
    let Value::List(nested) = &**value else {
        panic!("typed aggregate was not preserved");
    };
    assert_eq!(nested[0].as_f64().unwrap().to_bits(), (-0.0f64).to_bits());
    assert_eq!(nested[1], Value::Integer(7));
}

//! The `Scale` and `FactorX/Y/Z` writers refuse exactly what the reader
//! refuses (#254).
//!
//! The reader rejects a `Scale` that is zero, negative or non-finite
//! (`GeorefError::InvalidScale`) and, on `IfcMapConversionScaled`, a factor
//! that is (`GeorefError::InvalidAttribute`). A writer that accepted those
//! would stage a record this crate cannot read back. Each case here checks
//! both halves: the writer refuses with `AuthoringInvalid` naming the
//! attribute and stages nothing, and the same value written past the
//! writer is refused by the reader -- through a STEP round trip when the
//! value is finite (STEP has no spelling for NaN or infinity), in memory
//! otherwise. A positive value at the boundary round-trips and resolves.

use ifc_georef::{
    create_map_conversion, create_map_conversion_scaled, create_projected_crs,
    resolve_project_to_map, GeorefError, MapConversionDraft, OperationKind, ProjectedCrsDraft,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

/// Values the reader refuses for `Scale` and for every factor.
const REFUSED: [f64; 6] = [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

/// A positive value close to zero: still accepted by both sides.
const SMALLEST_ACCEPTED: f64 = 1e-9;

const FACTORS: [&str; 3] = ["FactorX", "FactorY", "FactorZ"];

/// A model holding a context and a projected CRS, with a transaction open.
fn setup() -> (Model, Transaction, EntityId, EntityId) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let source = tx.create(Entity::new(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![Value::Null; 6],
    ));
    let target = create_projected_crs(&mut tx, ProjectedCrsDraft::new("EPSG:25832"))
        .expect("a named projected CRS");
    tx.commit(&mut model).expect("commit");
    let tx = Transaction::new(&model);
    (model, tx, source, target)
}

fn draft(source: EntityId, target: EntityId) -> MapConversionDraft {
    MapConversionDraft::new(source, target, 400_000.0, 5_600_000.0, 112.5).x_axis((0.8, 0.6))
}

fn factors_with(position: usize, value: f64) -> (f64, f64, f64) {
    let mut factors = [1.0, 1.0, 1.0];
    factors[position] = value;
    (factors[0], factors[1], factors[2])
}

/// Write `model` to STEP and read it back.
fn round_trip(model: &Model) -> Model {
    let mut bytes = Vec::new();
    StepCodec.write(model, &mut bytes).expect("written");
    StepCodec.read_bytes(&bytes).expect("read back")
}

/// Bypass the writer: stage the record exactly as the writer would have,
/// commit it, and return the model the reader sees. Finite values go
/// through STEP; non-finite ones stay in memory.
fn written_past_the_writer(
    mut model: Model,
    type_name: &str,
    attributes: Vec<Value>,
    finite: bool,
) -> (Model, EntityId) {
    let mut tx = Transaction::new(&model);
    let id = tx.create(Entity::new(type_name, attributes));
    tx.commit(&mut model)
        .expect("the model layer does not check scales");
    if finite {
        (round_trip(&model), id)
    } else {
        (model, id)
    }
}

fn conversion_attributes(source: EntityId, target: EntityId, scale: f64) -> Vec<Value> {
    vec![
        Value::Ref(source),
        Value::Ref(target),
        Value::Real(400_000.0),
        Value::Real(5_600_000.0),
        Value::Real(112.5),
        Value::Real(0.8),
        Value::Real(0.6),
        Value::Real(scale),
    ]
}

fn assert_authoring_refusal(
    result: Result<EntityId, GeorefError>,
    expected_entity: &str,
    expected_attribute: &str,
    value: f64,
) {
    match result {
        Err(GeorefError::AuthoringInvalid {
            entity, attribute, ..
        }) => {
            assert_eq!(entity, expected_entity, "value {value}");
            assert_eq!(attribute, expected_attribute, "value {value}");
        }
        other => panic!("{expected_entity}.{expected_attribute} = {value}: {other:?}"),
    }
}

/// `Scale` on `IfcMapConversion`.
#[test]
fn the_map_conversion_writer_refuses_every_scale_the_reader_refuses() {
    for value in REFUSED {
        let (model, mut tx, source, target) = setup();
        let result = create_map_conversion(&mut tx, &model, draft(source, target).scale(value));
        assert_authoring_refusal(result, "IFCMAPCONVERSION", "Scale", value);
        assert!(tx.is_empty(), "Scale = {value}: nothing staged");

        let (read, id) = written_past_the_writer(
            model,
            "IFCMAPCONVERSION",
            conversion_attributes(source, target, value),
            value.is_finite(),
        );
        assert!(
            matches!(
                resolve_project_to_map(&read, id, 1.0),
                Err(GeorefError::InvalidScale { entity, .. }) if entity == id
            ),
            "the reader refuses Scale = {value}"
        );
    }
}

/// `Scale` on `IfcMapConversionScaled`, which shares the inherited checks.
#[test]
fn the_scaled_writer_refuses_every_scale_the_reader_refuses() {
    for value in REFUSED {
        let (model, mut tx, source, target) = setup();
        let result = create_map_conversion_scaled(
            &mut tx,
            &model,
            draft(source, target).scale(value),
            (1.0, 1.0, 1.0),
        );
        assert_authoring_refusal(result, "IFCMAPCONVERSIONSCALED", "Scale", value);
        assert!(tx.is_empty(), "Scale = {value}: nothing staged");

        let mut attributes = conversion_attributes(source, target, value);
        attributes.extend([Value::Real(1.0), Value::Real(1.0), Value::Real(1.0)]);
        let (read, id) = written_past_the_writer(
            model,
            "IFCMAPCONVERSIONSCALED",
            attributes,
            value.is_finite(),
        );
        assert!(
            matches!(
                resolve_project_to_map(&read, id, 1.0),
                Err(GeorefError::InvalidScale { entity, .. }) if entity == id
            ),
            "the reader refuses Scale = {value}"
        );
    }
}

/// `FactorX`, `FactorY` and `FactorZ` on `IfcMapConversionScaled`.
#[test]
fn the_scaled_writer_refuses_every_factor_the_reader_refuses() {
    for (position, name) in FACTORS.into_iter().enumerate() {
        for value in REFUSED {
            let (model, mut tx, source, target) = setup();
            let result = create_map_conversion_scaled(
                &mut tx,
                &model,
                draft(source, target).scale(1.0),
                factors_with(position, value),
            );
            assert_authoring_refusal(result, "IFCMAPCONVERSIONSCALED", name, value);
            assert!(tx.is_empty(), "{name} = {value}: nothing staged");

            let (x, y, z) = factors_with(position, value);
            let mut attributes = conversion_attributes(source, target, 1.0);
            attributes.extend([Value::Real(x), Value::Real(y), Value::Real(z)]);
            let (read, id) = written_past_the_writer(
                model,
                "IFCMAPCONVERSIONSCALED",
                attributes,
                value.is_finite(),
            );
            let expected_index = 8 + position;
            assert!(
                matches!(
                    resolve_project_to_map(&read, id, 1.0),
                    Err(GeorefError::InvalidAttribute { entity, index, name: read_name })
                        if entity == id && index == expected_index && read_name == name
                ),
                "the reader refuses {name} = {value}"
            );
        }
    }
}

/// The smallest positive values the writers accept read back and resolve.
///
/// One model per operation: a context carries at most one coordinate
/// operation (`HasCoordinateOperation : SET [0:1]`).
#[test]
fn a_small_positive_scale_and_factor_round_trip() {
    let (mut model, mut tx, source, target) = setup();
    let plain = create_map_conversion(
        &mut tx,
        &model,
        draft(source, target).scale(SMALLEST_ACCEPTED),
    )
    .expect("a positive scale is accepted");
    tx.commit(&mut model).expect("commit");
    let read = round_trip(&model);
    let plain = resolve_project_to_map(&read, plain, 1.0).expect("the scale reads back");
    assert_eq!(plain.declared_scale, SMALLEST_ACCEPTED);

    let (mut model, mut tx, source, target) = setup();
    let scaled = create_map_conversion_scaled(
        &mut tx,
        &model,
        draft(source, target).scale(1.0),
        (SMALLEST_ACCEPTED, SMALLEST_ACCEPTED, SMALLEST_ACCEPTED),
    )
    .expect("positive factors are accepted");
    tx.commit(&mut model).expect("commit");
    let read = round_trip(&model);
    let scaled = resolve_project_to_map(&read, scaled, 1.0).expect("the factors read back");
    assert_eq!(
        scaled.kind,
        OperationKind::MapConversionScaled {
            factors: (SMALLEST_ACCEPTED, SMALLEST_ACCEPTED, SMALLEST_ACCEPTED)
        }
    );
}

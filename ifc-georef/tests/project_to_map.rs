use std::sync::Arc;

use axiolid_core::{Point3, Vec3};
use ifc_georef::{resolve_project_to_map, GeorefError};
use ifc_model::value::Value;
use ifc_model::{Codec, Entity, EntityId, Model};
use ifc_step::StepCodec;

fn id(value: u64) -> EntityId {
    EntityId(value)
}

fn r(value: u64) -> Value {
    Value::Ref(id(value))
}

fn e(value: &str) -> Value {
    Value::Enum(Arc::from(value))
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

fn model_with_map_conversion(axis: (Value, Value), scale: Value) -> Model {
    let mut model = Model::new();
    model.insert(
        id(1),
        Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
    );
    model.insert(
        id(2),
        Entity::new(
            "IFCPROJECTEDCRS",
            vec![
                text("EPSG:25832"),
                text("ETRS89 / UTM zone 32N"),
                text("ETRS89"),
                text("DHHN2016"),
                text("UTM"),
                text("32N"),
                r(3),
            ],
        ),
    );
    model.insert(
        id(3),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, e("LENGTHUNIT"), Value::Null, e("METRE")],
        ),
    );
    model.insert(
        id(4),
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                r(1),
                r(2),
                Value::Real(1000.0),
                Value::Real(2000.0),
                Value::Real(50.0),
                axis.0,
                axis.1,
                scale,
            ],
        ),
    );
    model
}

#[test]
fn resolves_ifc4_map_conversion_into_a_metres_to_metres_transform() {
    let model = model_with_map_conversion((Value::Real(0.0), Value::Real(2.0)), Value::Real(2.0));

    let operation = resolve_project_to_map(&model, id(4), 1.0).expect("valid conversion");
    let actual = operation
        .transform
        .transform_point3(Point3::new(3.0, 4.0, 5.0));

    assert_eq!(actual, Vec3::new(992.0, 2006.0, 60.0));
    assert_eq!(operation.source_crs, id(1));
    assert_eq!(operation.target_crs.entity, id(2));
    assert_eq!(operation.target_crs.name, "EPSG:25832");
    assert_eq!(operation.map_unit.metres_per_unit, 1.0);
}

#[test]
fn converts_map_translation_and_scale_when_project_and_map_units_differ() {
    let mut model = model_with_map_conversion((Value::Null, Value::Null), Value::Real(0.001));
    model.insert(
        id(3),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, e("LENGTHUNIT"), e("MILLI"), e("METRE")],
        ),
    );

    // Project coordinates were authored in millimetres and are already metres
    // at the neutral boundary. Map coordinates are also millimetres. IFC Scale
    // converts source millimetres to target millimetres here.
    let operation = resolve_project_to_map(&model, id(4), 0.001).expect("valid conversion");
    let actual = operation
        .transform
        .transform_point3(Point3::new(3.0, 4.0, 5.0));

    assert_eq!(actual, Vec3::new(1.003, 2.004, 0.055));
    assert_eq!(operation.map_unit.metres_per_unit, 0.001);
}

#[test]
fn defaults_each_missing_axis_component_independently() {
    let abscissa_only =
        model_with_map_conversion((Value::Real(2.0), Value::Null), Value::Real(1.0));
    let operation = resolve_project_to_map(&abscissa_only, id(4), 1.0).expect("ordinate defaults");
    let mapped = operation
        .transform
        .transform_point3(Point3::new(3.0, 4.0, 0.0));
    assert_eq!(mapped, Vec3::new(1003.0, 2004.0, 50.0));

    let ordinate_only =
        model_with_map_conversion((Value::Null, Value::Real(1.0)), Value::Real(1.0));
    let operation = resolve_project_to_map(&ordinate_only, id(4), 1.0).expect("abscissa defaults");
    let mapped = operation
        .transform
        .transform_point3(Point3::new(3.0, 4.0, 0.0));
    let root_two = 2.0_f64.sqrt();
    assert!((mapped.x - (1000.0 - 1.0 / root_two)).abs() < 1e-12);
    assert!((mapped.y - (2000.0 + 7.0 / root_two)).abs() < 1e-12);
    assert_eq!(mapped.z, 50.0);
}

#[test]
fn refuses_map_conversion_scaled_until_unequal_factors_are_represented() {
    let mut model = model_with_map_conversion((Value::Null, Value::Null), Value::Real(1.0));
    model.insert(
        id(4),
        Entity::new(
            "IFCMAPCONVERSIONSCALED",
            vec![
                r(1),
                r(2),
                Value::Real(1000.0),
                Value::Real(2000.0),
                Value::Real(50.0),
                Value::Null,
                Value::Null,
                Value::Real(1.0),
                Value::Real(2.0),
                Value::Real(3.0),
                Value::Real(4.0),
            ],
        ),
    );

    let error = resolve_project_to_map(&model, id(4), 1.0).expect_err("scaled subtype is explicit");
    assert!(matches!(
        error,
        GeorefError::UnsupportedOperation { entity, actual }
            if entity == id(4) && actual == "IFCMAPCONVERSIONSCALED"
    ));
}

#[test]
fn rejects_a_zero_axis_and_non_positive_scale() {
    let zero_axis =
        model_with_map_conversion((Value::Real(0.0), Value::Real(0.0)), Value::Real(1.0));
    assert!(matches!(
        resolve_project_to_map(&zero_axis, id(4), 1.0),
        Err(GeorefError::DegenerateAxis { entity }) if entity == id(4)
    ));

    let negative_scale = model_with_map_conversion((Value::Null, Value::Null), Value::Real(-1.0));
    assert!(matches!(
        resolve_project_to_map(&negative_scale, id(4), 1.0),
        Err(GeorefError::InvalidScale { entity, .. }) if entity == id(4)
    ));
}

#[test]
fn rejects_overflow_in_the_unit_normalized_scale() {
    let mut model = model_with_map_conversion((Value::Null, Value::Null), Value::Real(1.0e308));
    model.insert(
        id(3),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, e("LENGTHUNIT"), e("EXA"), e("METRE")],
        ),
    );

    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0e-18),
        Err(GeorefError::InvalidScale { entity, value })
            if entity == id(4) && !value.is_finite()
    ));
}

#[test]
fn rejects_overflow_in_the_unit_normalized_translation() {
    let mut model = model_with_map_conversion((Value::Null, Value::Null), Value::Real(1.0));
    model.insert(
        id(3),
        Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, e("LENGTHUNIT"), e("EXA"), e("METRE")],
        ),
    );
    model.insert(
        id(4),
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                r(1),
                r(2),
                Value::Real(1.0e308),
                Value::Real(2.0),
                Value::Real(3.0),
                Value::Null,
                Value::Null,
                Value::Real(1.0),
            ],
        ),
    );

    assert!(matches!(
        resolve_project_to_map(&model, id(4), 1.0),
        Err(GeorefError::InvalidAttribute {
            entity,
            index: 2,
            name: "Eastings"
        }) if entity == id(4)
    ));
}

#[test]
fn resolves_a_committed_ifc_fixture_to_the_neutral_map_transform() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_conic_offset_bounded.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let operation = resolve_project_to_map(&model, id(51), 1.0).expect("conversion resolves");
    let mapped = operation
        .transform
        .transform_point3(Point3::new(1.0, 2.0, 3.0));
    assert_eq!(mapped, Vec3::new(2.0, 4.0, 3.01));
    assert_eq!(operation.target_crs.name, "EPSG:25832");
}

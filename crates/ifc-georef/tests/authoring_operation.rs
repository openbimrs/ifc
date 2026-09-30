//! Coordinate operations: map conversions and rigid shifts.
//!
//! The rule under test is `TargetCRSOnlyProjected`. Eastings and
//! northings are coordinates on a projection plane, so aiming them at
//! a geographic CRS produces numbers that parse and denote nothing.

use axiolid_core::{Point3, Vec3};
use ifc_georef::{
    create_angular_rigid_operation, create_geographic_crs, create_map_conversion,
    create_map_conversion_scaled, create_projected_crs, create_rigid_operation,
    resolve_geographic_offset_in, resolve_project_to_map, GeographicCrsDraft, GeorefView,
    MapConversionDraft, ProjectedCrsDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn context(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![Value::Null; 6],
    ))
}

fn projected(tx: &mut Transaction) -> EntityId {
    create_projected_crs(tx, ProjectedCrsDraft::new("EPSG:25832"))
        .expect("a named projected CRS is accepted")
}

fn draft(source: EntityId, target: EntityId) -> MapConversionDraft {
    MapConversionDraft::new(source, target, 400_000.0, 5_600_000.0, 112.5)
        .x_axis((0.8, 0.6))
        .scale(1.0)
}

/// A map conversion refuses a target that is not a projected CRS.
///
/// This is `TargetCRSOnlyProjected`. A geographic CRS has degrees on
/// its axes, so a 400000.0 easting against one is off the planet.
#[test]
fn a_map_conversion_requires_a_projected_target() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let source = context(&mut tx);
    let geographic = create_geographic_crs(&mut tx, GeographicCrsDraft::new().name("EPSG:4326"))
        .expect("a named geographic CRS is accepted");

    assert!(
        create_map_conversion(&mut tx, &model, draft(source, geographic)).is_err(),
        "eastings and northings are meaningless against a degree-based CRS"
    );

    let target = projected(&mut tx);
    let id = create_map_conversion(&mut tx, &model, draft(source, target))
        .expect("a projected target is accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCMAPCONVERSION");
    assert_eq!(entity.attributes[2], Value::Real(400_000.0), "Eastings");
    assert_eq!(entity.attributes[3], Value::Real(5_600_000.0), "Northings");
    assert_eq!(entity.attributes[4], Value::Real(112.5), "OrthogonalHeight");
    assert_eq!(
        entity.attributes[5],
        Value::Real(0.8),
        "XAxisAbscissa keeps its own component"
    );
    assert_eq!(
        entity.attributes[6],
        Value::Real(0.6),
        "XAxisOrdinate is not the abscissa"
    );
}

/// A zero-length X axis gives no rotation, and is refused.
///
/// The two reals are the components of a direction vector, not a
/// bearing, so `(0, 0)` is not "no rotation" -- it is no direction.
#[test]
fn a_zero_length_x_axis_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let source = context(&mut tx);
    let target = projected(&mut tx);

    assert!(
        create_map_conversion(&mut tx, &model, draft(source, target).x_axis((0.0, 0.0)),).is_err(),
        "a zero-length vector has no direction"
    );

    assert!(
        create_map_conversion(
            &mut tx,
            &model,
            draft(source, target).x_axis((f64::NAN, 1.0)),
        )
        .is_err(),
        "a non-finite component is refused"
    );

    assert!(
        create_map_conversion(&mut tx, &model, draft(source, target).scale(0.0),).is_err(),
        "a zero scale collapses the model"
    );

    assert!(
        create_map_conversion(&mut tx, &model, {
            let mut infinite = draft(source, target);
            infinite.eastings = f64::INFINITY;
            infinite
        },)
        .is_err(),
        "a non-finite easting is refused"
    );
}

/// Per-axis factors land after the eight inherited attributes.
#[test]
fn scaled_factors_follow_the_inherited_attributes() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let source = context(&mut tx);
    let target = projected(&mut tx);

    assert!(
        create_map_conversion_scaled(&mut tx, &model, draft(source, target), (1.0, 0.0, 1.0))
            .is_err(),
        "a zero factor collapses that axis"
    );

    let id = create_map_conversion_scaled(
        &mut tx,
        &model,
        draft(source, target),
        (0.9996, 0.9997, 1.0),
    )
    .expect("non-zero finite factors are accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCMAPCONVERSIONSCALED");
    assert_eq!(entity.attributes.len(), 11);
    assert_eq!(entity.attributes[8], Value::Real(0.9996), "FactorX");
    assert_eq!(entity.attributes[9], Value::Real(0.9997), "FactorY");
    assert_eq!(entity.attributes[10], Value::Real(1.0), "FactorZ");
}

/// A rigid operation needs no projected target, but its coordinates must
/// be typed.
///
/// It is a translation, not a projection, so the writer does not demand a
/// projected CRS. `FirstCoordinate` and `SecondCoordinate` are
/// `IfcMeasureValue` selects under `SameCoordinateType`: an untyped REAL
/// satisfies neither branch, so lengths are written `IFCLENGTHMEASURE(..)`
/// and latitude/longitude offsets `IFCPLANEANGLEMEASURE(..)`. Both read
/// back through the resolvers.
#[test]
fn a_rigid_operation_accepts_any_target() {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);

    let source = context(&mut tx);
    let target = projected(&mut tx);
    let geographic = create_geographic_crs(&mut tx, GeographicCrsDraft::new().name("EPSG:4326"))
        .expect("accepted");

    assert!(
        create_rigid_operation(&mut tx, source, target, (f64::NAN, 2.0), None).is_err(),
        "a non-finite coordinate is refused"
    );
    assert!(
        create_angular_rigid_operation(&mut tx, source, geographic, (1.0, 2.0), Some(f64::NAN))
            .is_err(),
        "a non-finite height is refused"
    );

    let length = create_rigid_operation(&mut tx, source, target, (10.0, 20.0), Some(3.0))
        .expect("a length shift onto a projected CRS");
    let second = context(&mut tx);
    let angular = create_angular_rigid_operation(&mut tx, second, geographic, (0.5, 0.25), None)
        .expect("an angular shift onto a geographic CRS");

    tx.commit(&mut model).expect("commit");

    let typed = |measure: &str, value: f64| Value::Typed {
        type_name: measure.into(),
        value: Box::new(Value::Real(value)),
    };
    let entity = model.get(length).expect("staged");
    assert_eq!(entity.type_name.as_ref(), "IFCRIGIDOPERATION");
    assert_eq!(entity.attributes[2], typed("IFCLENGTHMEASURE", 10.0));
    assert_eq!(entity.attributes[3], typed("IFCLENGTHMEASURE", 20.0));
    assert_eq!(
        entity.attributes[4],
        Value::Real(3.0),
        "Height is not a select"
    );
    let entity = model.get(angular).expect("staged");
    assert_eq!(entity.attributes[2], typed("IFCPLANEANGLEMEASURE", 0.5));
    assert_eq!(entity.attributes[3], typed("IFCPLANEANGLEMEASURE", 0.25));
    assert_eq!(entity.attributes[4], Value::Null);

    let resolved = resolve_project_to_map(&model, length, 1.0).expect("length form resolves");
    assert_eq!(
        resolved.transform.transform_point3(Point3::ZERO),
        Vec3::new(10.0, 20.0, 3.0)
    );
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    let offset = resolve_geographic_offset_in(&view, angular).expect("angular form reads");
    assert_eq!(
        (offset.first_coordinate, offset.second_coordinate),
        (0.5, 0.25)
    );
}

/// A geographic CRS with a blank name identifies nothing.
#[test]
fn a_blank_crs_name_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    assert!(
        create_geographic_crs(&mut tx, GeographicCrsDraft::new().name("   "),).is_err(),
        "a blank name identifies no CRS"
    );

    let id = create_geographic_crs(
        &mut tx,
        GeographicCrsDraft::new()
            .name("EPSG:4979")
            .geodetic_datum("WGS84"),
    )
    .expect("accepted");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("staged");
    assert_eq!(entity.attributes[0], Value::Text("EPSG:4979".into()));
    assert_eq!(entity.attributes[2], Value::Text("WGS84".into()));
}

//! Integration coverage for GEOREF-VERS, GEOREF-CHAIN, and GEOREF-NORTH:
//! schema-pinned resolution, project-frame composition, and true/grid/
//! project north, exercised together against realistic model shapes.

use std::sync::Arc;

use axiolid_core::{Mat3, Point3, Transform3, Vec3};
use ifc_georef::{
    compose_project_frame, grid_north_direction, project_north_direction, resolve_project_to_map,
    resolve_project_to_map_in, resolve_true_north, GeorefError, GeorefView, NorthReference,
};
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

fn id(value: u64) -> EntityId {
    EntityId(value)
}

fn r(value: u64) -> Value {
    Value::Ref(id(value))
}

fn real(value: f64) -> Value {
    Value::Real(value)
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

/// A minimal IFC4/IFC4X3 georeferencing chain: a geometric representation
/// context (with an explicit `TrueNorth`) linked to a projected CRS through
/// a map conversion whose local X axis is rotated 90 degrees onto the map's
/// northing axis.
fn schema_pinned_model(schema_token: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema_token.to_owned()];
    model.insert(
        id(10),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![real(0.5), real(0.8660254037844387)])],
        ),
    );
    model.insert(
        id(1),
        Entity::new(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            vec![
                Value::Null,
                Value::Null,
                Value::Integer(3),
                Value::Null,
                Value::Null,
                r(10),
            ],
        ),
    );
    model.insert(
        id(2),
        Entity::new("IFCPROJECTEDCRS", vec![text("EPSG:25832")]),
    );
    model.insert(
        id(4),
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                r(1),
                r(2),
                real(500000.0),
                real(5800000.0),
                real(100.0),
                real(0.0),
                real(1.0),
                real(1.0),
            ],
        ),
    );
    model
}

#[test]
fn georef_view_pins_ifc4_and_resolves_a_map_conversion_through_it() {
    let model = schema_pinned_model("IFC4");
    let view = GeorefView::for_model(&model).expect("IFC4 is a valid georeferencing profile");

    let operation = resolve_project_to_map_in(&view, id(4), 1.0).expect("resolves under IFC4");
    assert_eq!(operation.target_crs.name, "EPSG:25832");

    // Both entry points agree: the schema-agnostic resolver is not a
    // shortcut that skips validation, it is the same underlying resolution.
    let unpinned = resolve_project_to_map(&model, id(4), 1.0).expect("resolves without a view");
    assert_eq!(operation.transform, unpinned.transform);
}

#[test]
fn georef_view_pins_ifc4x3_identically() {
    let model = schema_pinned_model("IFC4X3_ADD2");
    let view = GeorefView::for_model(&model).expect("IFC4X3 ADD2 is accepted");
    assert_eq!(view.version(), ifc_schema::SchemaVersion::Ifc4x3);
    resolve_project_to_map_in(&view, id(4), 1.0).expect("resolves under IFC4X3");
}

#[test]
fn resolving_an_ifc4x3_only_entity_under_ifc4_names_the_schema_mismatch() {
    let mut model = schema_pinned_model("IFC4");
    model.insert(id(4), Entity::new("IFCGEOGRAPHICCRS", vec![]));
    let view = GeorefView::for_model(&model).expect("IFC4 is accepted");

    let error = resolve_project_to_map_in(&view, id(4), 1.0)
        .expect_err("IfcGeographicCRS is not declared in IFC4 at all");
    assert!(matches!(
        error,
        GeorefError::UnsupportedOperation { entity, actual }
            if entity == id(4) && actual.contains("IFCGEOGRAPHICCRS") && actual.contains("IFC4_ADD2_TC1")
    ));
}

#[test]
fn compose_project_frame_chains_onto_the_pinned_view_s_resolved_operation() {
    let model = schema_pinned_model("IFC4X3");
    let view = GeorefView::for_model(&model).expect("IFC4X3 is accepted");
    let operation = resolve_project_to_map_in(&view, id(4), 1.0).expect("resolves");

    let project_frame = Transform3::from_mat3_translation(Mat3::IDENTITY, Vec3::new(1.0, 2.0, 3.0));
    let map_frame = compose_project_frame(&operation, project_frame).expect("composes");

    let expected = operation
        .transform
        .transform_point3(Point3::new(1.0, 2.0, 3.0));
    let actual = map_frame.transform_point3(Point3::ZERO);
    assert_eq!(actual, expected);
}

#[test]
fn true_grid_and_project_north_are_distinguishable_and_can_disagree() {
    let model = schema_pinned_model("IFC4X3");
    let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");

    let project = NorthReference::Project;
    let true_north = resolve_true_north(&model, id(1)).expect("TrueNorth resolves");
    let grid_north = grid_north_direction(&operation);

    // Project north is always (0, 1) by definition.
    assert_eq!(project.direction(), project_north_direction());

    // True north was declared as a 30-degree bearing off project north.
    let (tx, ty) = true_north.direction();
    assert!((tx - 0.5).abs() < 1e-9);
    assert!((ty - 0.8660254037844387).abs() < 1e-9);

    // Grid north was rotated a full 90 degrees by the map conversion's own
    // XAxisAbscissa/XAxisOrdinate -- a different value again, because IFC
    // allows true north and grid north to disagree with each other and with
    // the map conversion's own implied rotation. Per the schema note in
    // `north/directions.rs`, grid north (from IfcMapConversion) is
    // authoritative for georeferencing when both are present.
    let (gx, gy) = grid_north.direction();
    assert!((gx - 1.0).abs() < 1e-9);
    assert!(gy.abs() < 1e-9);

    assert_ne!(true_north.direction(), grid_north.direction());
    assert!(matches!(true_north, NorthReference::True { .. }));
    assert!(matches!(grid_north, NorthReference::Grid { .. }));
}

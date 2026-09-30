//! IFC4X3 `IfcRigidOperation`, `IfcMapConversionScaled` and
//! `IfcGeographicCRS` (#241), and their refusal under IFC4.

#[path = "support/step.rs"]
mod support;

use axiolid_core::{Point3, Vec3};
use ifc_georef::{
    grid_north_direction, resolve_geographic_offset_in, resolve_project_to_map,
    resolve_project_to_map_in, GeorefError, GeorefView, OperationKind, OperationSource,
};
use ifc_model::{EntityId, Model};
use support::{step, BASE, GEOGRAPHIC};

const OPERATION: EntityId = EntityId(80);

fn ifc4x3(operation: &str) -> Model {
    step("IFC4X3_ADD2", &format!("{BASE}{GEOGRAPHIC}{operation}\n"))
}

fn close(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() < 1e-9,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn a_length_rigid_operation_resolves_to_a_translation_with_height() {
    let model = ifc4x3(
        "#80=IFCRIGIDOPERATION(#7,#50,IFCLENGTHMEASURE(500000.),IFCLENGTHMEASURE(5800000.),100.);",
    );
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    let operation = resolve_project_to_map_in(&view, OPERATION, 1.0).expect("resolves");
    close(
        operation
            .transform
            .transform_point3(Point3::new(1.0, 2.0, 3.0)),
        Vec3::new(500_001.0, 5_800_002.0, 103.0),
    );
    assert_eq!(
        operation.kind,
        OperationKind::RigidOperation {
            height: Some(100.0)
        }
    );
    assert_eq!(operation.source, OperationSource::Context(EntityId(7)));
    assert_eq!(operation.operation, OPERATION);
    assert_eq!(operation.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(operation.x_axis_direction, (1.0, 0.0));

    // The unpinned entry point pins the header itself and agrees.
    let unpinned = resolve_project_to_map(&model, OPERATION, 1.0).expect("resolves");
    assert_eq!(unpinned, operation);
}

#[test]
fn a_rigid_operation_preserves_distances_whatever_the_project_unit() {
    // Millimetre project, metre map: the offset is in the map unit, and
    // the linear part stays the identity in metres.
    let model = ifc4x3(
        "#80=IFCRIGIDOPERATION(#7,#50,IFCLENGTHMEASURE(10.),IFCPOSITIVELENGTHMEASURE(20.),$);",
    );
    let operation = resolve_project_to_map(&model, OPERATION, 0.001).expect("resolves");
    close(
        operation
            .transform
            .transform_point3(Point3::new(1.0, 2.0, 3.0)),
        Vec3::new(11.0, 22.0, 3.0),
    );
    assert_eq!(
        operation.kind,
        OperationKind::RigidOperation { height: None },
        "an unstated height is recorded as unstated, not as zero"
    );
}

#[test]
fn same_coordinate_type_is_checked() {
    for (coordinates, why) in [
        ("10.,20.", "untyped REALs satisfy neither branch"),
        (
            "IFCLENGTHMEASURE(10.),IFCPLANEANGLEMEASURE(0.1)",
            "a length and an angle are mixed",
        ),
        (
            "IFCRATIOMEASURE(1.),IFCRATIOMEASURE(2.)",
            "a ratio is neither a length nor an angle",
        ),
    ] {
        let model = ifc4x3(&format!("#80=IFCRIGIDOPERATION(#7,#50,{coordinates},$);"));
        assert!(
            matches!(
                resolve_project_to_map(&model, OPERATION, 1.0),
                Err(GeorefError::RuleViolation { entity, rule: "SameCoordinateType" })
                    if entity == OPERATION
            ),
            "{why}"
        );
    }
}

#[test]
fn a_length_rigid_operation_needs_a_projected_target() {
    let model =
        ifc4x3("#80=IFCRIGIDOPERATION(#7,#70,IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.),$);");
    assert!(matches!(
        resolve_project_to_map(&model, OPERATION, 1.0),
        Err(GeorefError::WrongType { entity, expected: "IFCPROJECTEDCRS", .. })
            if entity == EntityId(70)
    ));
}

#[test]
fn a_plane_angle_rigid_operation_is_read_as_a_geographic_offset() {
    let model = ifc4x3(
        "#80=IFCRIGIDOPERATION(#7,#70,IFCPLANEANGLEMEASURE(50.98),IFCPOSITIVEPLANEANGLEMEASURE(11.32),210.);",
    );
    assert!(
        matches!(
            resolve_project_to_map(&model, OPERATION, 1.0),
            Err(GeorefError::CoordinateMeasureMismatch {
                entity,
                expected: "IFCLENGTHMEASURE",
                actual: "IFCPLANEANGLEMEASURE",
            }) if entity == OPERATION
        ),
        "a latitude/longitude offset has no metre transform"
    );

    let view = GeorefView::for_model(&model).expect("IFC4X3");
    let offset = resolve_geographic_offset_in(&view, OPERATION).expect("reads");
    assert_eq!(offset.first_coordinate, 50.98);
    assert_eq!(offset.second_coordinate, 11.32);
    assert_eq!(offset.height, Some(210.0));
    assert_eq!(offset.source, OperationSource::Context(EntityId(7)));
    let crs = &offset.target_crs;
    assert_eq!(crs.entity, EntityId(70));
    assert_eq!(crs.name.as_deref(), Some("EPSG:4979"));
    assert_eq!(crs.geodetic_datum.as_deref(), Some("WGS84"));
    assert_eq!(crs.prime_meridian.as_deref(), Some("Greenwich"));
    let angle = crs.angle_unit.as_ref().expect("AngleUnit is stated");
    assert_eq!(angle.name, "DEGREE");
    assert!((angle.radians_per_unit - std::f64::consts::PI / 180.0).abs() < 1e-15);
    let height = crs.height_unit.as_ref().expect("HeightUnit is stated");
    assert_eq!(height.metres_per_unit, 1.0);
}

#[test]
fn the_geographic_reader_refuses_lengths_and_projected_targets() {
    let lengths =
        ifc4x3("#80=IFCRIGIDOPERATION(#7,#50,IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.),$);");
    let view = GeorefView::for_model(&lengths).expect("IFC4X3");
    assert!(matches!(
        resolve_geographic_offset_in(&view, OPERATION),
        Err(GeorefError::CoordinateMeasureMismatch {
            expected: "IFCPLANEANGLEMEASURE",
            actual: "IFCLENGTHMEASURE",
            ..
        })
    ));

    let projected = ifc4x3(
        "#80=IFCRIGIDOPERATION(#7,#50,IFCPLANEANGLEMEASURE(1.),IFCPLANEANGLEMEASURE(2.),$);",
    );
    let view = GeorefView::for_model(&projected).expect("IFC4X3");
    assert!(matches!(
        resolve_geographic_offset_in(&view, OPERATION),
        Err(GeorefError::WrongType { entity, expected: "IFCGEOGRAPHICCRS", .. })
            if entity == EntityId(50)
    ));
}

#[test]
fn a_geographic_angle_unit_must_be_a_plane_angle() {
    let model = step(
        "IFC4X3_ADD2",
        &format!(
            "{BASE}#70=IFCGEOGRAPHICCRS('EPSG:4979',$,$,$,#1,$);\n\
             #80=IFCRIGIDOPERATION(#7,#70,IFCPLANEANGLEMEASURE(1.),IFCPLANEANGLEMEASURE(2.),$);\n"
        ),
    );
    let view = GeorefView::for_model(&model).expect("IFC4X3");
    assert!(
        matches!(
            resolve_geographic_offset_in(&view, OPERATION),
            Err(GeorefError::InvalidUnit { entity, detail: "PLANEANGLEUNIT" })
                if entity == EntityId(1)
        ),
        "AngleUnitIsPlaneAngle"
    );
}

#[test]
fn a_scaled_map_conversion_resolves_with_per_axis_factors() {
    // X axis (0.6, 0.8), Scale 2, factors (0.5, 2, 4).
    let model = ifc4x3("#80=IFCMAPCONVERSIONSCALED(#7,#50,1000.,2000.,50.,0.6,0.8,2.,0.5,2.,4.);");
    let operation = resolve_project_to_map(&model, OPERATION, 1.0).expect("resolves");
    assert_eq!(
        operation.kind,
        OperationKind::MapConversionScaled {
            factors: (0.5, 2.0, 4.0)
        }
    );
    // E = 1000 + 2 * (0.6 * 0.5 * x - 0.8 * 2 * y), and so on.
    let (x, y, z) = (1.0, 1.0, 1.0);
    close(
        operation.transform.transform_point3(Point3::new(x, y, z)),
        Vec3::new(
            1000.0 + 2.0 * (0.6 * 0.5 * x - 0.8 * 2.0 * y),
            2000.0 + 2.0 * (0.8 * 0.5 * x + 0.6 * 2.0 * y),
            50.0 + 2.0 * 4.0 * z,
        ),
    );

    // Grid north, pushed through the operation, must point at map north.
    let (gx, gy) = grid_north_direction(&operation).direction();
    assert!((gx.hypot(gy) - 1.0).abs() < 1e-12, "normalized");
    let pushed = operation.transform.matrix3.mul_vec3(Vec3::new(gx, gy, 0.0));
    assert!(pushed.x.abs() < 1e-12 && pushed.y > 0.0, "{pushed:?}");
}

#[test]
fn a_scaled_map_conversion_refuses_a_non_positive_factor() {
    let model = ifc4x3("#80=IFCMAPCONVERSIONSCALED(#7,#50,1000.,2000.,50.,$,$,1.,1.,0.,1.);");
    assert!(matches!(
        resolve_project_to_map(&model, OPERATION, 1.0),
        Err(GeorefError::InvalidAttribute { entity, index: 9, name: "FactorY" })
            if entity == OPERATION
    ));
}

#[test]
fn ifc4_declares_none_of_the_ifc4x3_operations() {
    for operation in [
        "#80=IFCRIGIDOPERATION(#7,#50,IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.),$);",
        "#80=IFCMAPCONVERSIONSCALED(#7,#50,1000.,2000.,50.,$,$,1.,1.,1.,1.);",
    ] {
        let model = step("IFC4", &format!("{BASE}{operation}\n"));
        let view = GeorefView::for_model(&model).expect("IFC4");
        let undeclared = |result: Result<_, GeorefError>| {
            matches!(
                result,
                Err(GeorefError::UnsupportedOperation { entity, actual })
                    if entity == OPERATION && actual.contains("IFC4_ADD2_TC1")
            )
        };
        assert!(
            undeclared(resolve_project_to_map_in(&view, OPERATION, 1.0)),
            "{operation}"
        );
        assert!(
            undeclared(resolve_project_to_map(&model, OPERATION, 1.0)),
            "the unpinned entry point pins the IFC4 header too: {operation}"
        );
        assert!(
            matches!(
                resolve_geographic_offset_in(&view, OPERATION),
                Err(GeorefError::UnsupportedOperation { .. })
            ),
            "{operation}"
        );
    }
}

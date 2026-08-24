//! Lowering properties the IFC corpus alone cannot prove.

use axiolid_model::{GeometryNode, SolidOperation};
use axiolid_profile::Profile;
use ifc_geometry::lower::{lower_extruded_area_solid, lower_profile, LoweredGeometry, Tolerance};
use ifc_geometry::{Transform, UnitScale};
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn n(value: f64) -> Value {
    Value::Real(value)
}

fn millimetres() -> UnitScale {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum("LENGTHUNIT".into()),
                Value::Enum("MILLI".into()),
                Value::Enum("METRE".into()),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCUNITASSIGNMENT", vec![Value::List(vec![r(1)])]),
    );
    let mut project = vec![Value::Null; 9];
    project[8] = r(2);
    model.insert(EntityId(3), Entity::new("IFCPROJECT", project));
    ifc_geometry::units::resolve(&model)
}

fn extrusion_model(depth: f64, with_position: bool) -> (Model, EntityId) {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Null,
                n(100.0),
                n(200.0),
            ],
        ),
    );
    let position = if with_position {
        model.insert(
            EntityId(3),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(vec![n(1000.0), n(0.0), n(0.0)])],
            ),
        );
        model.insert(
            EntityId(4),
            Entity::new("IFCAXIS2PLACEMENT3D", vec![r(3), Value::Null, Value::Null]),
        );
        r(4)
    } else {
        Value::Null
    };
    model.insert(
        EntityId(5),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(2), position, r(1), n(depth)]),
    );
    (model, EntityId(5))
}

fn operation(lowered: &LoweredGeometry) -> &SolidOperation {
    let source = match lowered.graph.get(lowered.root).expect("root exists") {
        GeometryNode::Instance(instance) => instance.source,
        other => panic!("expected instance root, got {other:?}"),
    };
    match lowered.graph.get(source).expect("source exists") {
        GeometryNode::SolidOperation(operation) => operation,
        other => panic!("expected solid operation, got {other:?}"),
    }
}

#[test]
fn extrusion_depth_is_converted_to_metres() {
    let (model, id) = extrusion_model(2500.0, false);
    let lowered = lower_extruded_area_solid(
        &model,
        id,
        Transform::identity(),
        &millimetres(),
        &Tolerance::building_scale(),
    )
    .expect("lowers");
    match operation(&lowered) {
        SolidOperation::Extrusion { depth, .. } => assert!((depth - 2.5).abs() < 1e-12),
        other => panic!("expected extrusion, got {other:?}"),
    }
}

#[test]
fn profile_dimensions_are_converted_to_metres() {
    let (model, _) = extrusion_model(1.0, false);
    let profile = lower_profile(
        &model,
        EntityId(2),
        &millimetres(),
        &Tolerance::building_scale(),
    )
    .expect("lowers");
    match profile {
        Profile::Rectangle(rectangle) => {
            assert!((rectangle.x - 0.1).abs() < 1e-12);
            assert!((rectangle.y - 0.2).abs() < 1e-12);
        }
        other => panic!("expected rectangle, got {other:?}"),
    }
}

#[test]
fn solid_position_is_composed_into_instance_transform() {
    let tolerance = Tolerance::building_scale();
    let (with_position, with_id) = extrusion_model(1000.0, true);
    let (without_position, without_id) = extrusion_model(1000.0, false);
    let a = lower_extruded_area_solid(
        &with_position,
        with_id,
        Transform::identity(),
        &millimetres(),
        &tolerance,
    )
    .expect("lowers");
    let b = lower_extruded_area_solid(
        &without_position,
        without_id,
        Transform::identity(),
        &millimetres(),
        &tolerance,
    )
    .expect("lowers");

    let transform =
        |lowered: &LoweredGeometry| match lowered.graph.get(lowered.root).expect("root exists") {
            GeometryNode::Instance(instance) => instance.transform,
            other => panic!("expected instance, got {other:?}"),
        };
    let moved = transform(&a).transform_point3(axiolid_core::Point3::ZERO);
    assert!((moved.x - 1.0).abs() < 1e-12);
    assert_ne!(transform(&a), transform(&b));
}

#[test]
fn circles_remain_exact_and_tolerance_independent() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCIRCLEPROFILEDEF",
            vec![Value::Enum("AREA".into()), Value::Null, Value::Null, n(5.0)],
        ),
    );
    let fine = lower_profile(
        &model,
        EntityId(1),
        &UnitScale::default(),
        &Tolerance::building_scale(),
    )
    .expect("lowers");
    let coarse = lower_profile(
        &model,
        EntityId(1),
        &UnitScale::default(),
        &Tolerance::from_sagitta(0.05).expect("valid"),
    )
    .expect("lowers");
    assert_eq!(fine, coarse, "IFC lowering must not tessellate a circle");
    match fine {
        Profile::Circle(circle) => assert_eq!(circle.radius, 5.0),
        other => panic!("expected exact circle, got {other:?}"),
    }
}

#[test]
fn parameterized_profile_position_is_preserved() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(1000.0), n(2000.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCDIRECTION", vec![Value::List(vec![n(0.0), n(1.0)])]),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCAXIS2PLACEMENT2D", vec![r(1), r(2)]),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                r(3),
                n(100.0),
                n(200.0),
            ],
        ),
    );
    let profile = lower_profile(
        &model,
        EntityId(4),
        &millimetres(),
        &Tolerance::building_scale(),
    )
    .expect("lowers");
    match profile {
        Profile::Derived { transform, .. } => {
            let origin = transform.transform_point2(axiolid_core::Point2::ZERO);
            let x = transform.transform_vector2(axiolid_core::Vec2::X);
            assert!((origin.x - 1.0).abs() < 1e-12);
            assert!((origin.y - 2.0).abs() < 1e-12);
            assert!(x.x.abs() < 1e-12);
            assert!((x.y - 1.0).abs() < 1e-12);
        }
        other => panic!("expected positioned profile, got {other:?}"),
    }
}

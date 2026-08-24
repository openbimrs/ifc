//! End-to-end proof that the IFC layer emits a backend-neutral geometry DAG.

use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{lower_extruded_area_solid, Tolerance};
use ifc_geometry::resource::mapped::MappingWalker;
use ifc_geometry::{rules, select, Transform, UnitScale};
use ifc_model::{Entity, EntityId, Model, Value};

fn n(value: f64) -> Value {
    Value::Real(value)
}

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn list(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(n).collect())
}

#[test]
fn full_ifc_pipeline_emits_exact_neutral_nodes() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Null,
                n(0.3),
                n(4.0),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCDIRECTION", vec![list(&[0.0, 0.0, 1.0])]),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCEXTRUDEDAREASOLID",
            vec![r(1), Value::Null, r(2), n(2.41)],
        ),
    );

    assert!(select::is_a("IFCEXTRUDEDAREASOLID", "IFCSOLIDMODEL"));
    assert!(rules::validate(&model, EntityId(3)).is_empty());

    let lowered = lower_extruded_area_solid(
        &model,
        EntityId(3),
        Transform::identity(),
        &UnitScale::default(),
        &Tolerance::building_scale(),
    )
    .expect("lowering succeeds without a backend");

    let operation = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Instance(instance) => instance.source,
        other => panic!("expected instance, got {other:?}"),
    };
    match lowered.graph.get(operation).expect("operation") {
        GeometryNode::SolidOperation(SolidOperation::Extrusion { depth, .. }) => {
            assert_eq!(*depth, 2.41);
        }
        other => panic!("expected extrusion, got {other:?}"),
    }
}

#[test]
fn units_are_resolved_before_the_neutral_graph() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Null,
                n(300.0),
                n(4000.0),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCDIRECTION", vec![list(&[0.0, 0.0, 1.0])]),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCEXTRUDEDAREASOLID",
            vec![r(1), Value::Null, r(2), n(2410.0)],
        ),
    );
    let scale = UnitScale {
        length_to_metres: 1e-3,
        angle_to_radians: 1.0,
    };
    let lowered = lower_extruded_area_solid(
        &model,
        EntityId(3),
        Transform::identity(),
        &scale,
        &Tolerance::building_scale(),
    )
    .expect("lowers");
    let op = match lowered.graph.get(lowered.root).expect("root") {
        GeometryNode::Instance(instance) => lowered.graph.get(instance.source).expect("op"),
        other => panic!("expected instance, got {other:?}"),
    };
    assert!(matches!(
        op,
        GeometryNode::SolidOperation(SolidOperation::Extrusion { depth, .. })
            if (*depth - 2.41).abs() < 1e-12
    ));
}

#[test]
fn nested_placement_chain_composes_parent_first() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[0.0, 0.0, 3.0])]),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCLOCALPLACEMENT", vec![Value::Null, r(2)]),
    );
    model.insert(
        EntityId(4),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[5.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(5),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(4), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(6),
        Entity::new("IFCLOCALPLACEMENT", vec![r(3), r(5)]),
    );

    let mut resolver = ifc_geometry::constraint::local::PlacementResolver::new();
    let world = resolver
        .world_transform(&model, EntityId(6))
        .expect("resolves");
    assert_eq!(world.apply([0.0, 0.0, 0.0]), [5.0, 0.0, 3.0]);
}

#[test]
fn mapped_item_resolves_before_graph_instancing() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![list(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null]),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCSHAPEREPRESENTATION", vec![Value::Null]),
    );
    model.insert(
        EntityId(4),
        Entity::new("IFCREPRESENTATIONMAP", vec![r(2), r(3)]),
    );
    model.insert(
        EntityId(5),
        Entity::new("IFCCARTESIANTRANSFORMATIONOPERATOR3D", vec![Value::Null]),
    );
    model.insert(EntityId(6), Entity::new("IFCMAPPEDITEM", vec![r(4), r(5)]));

    let mut walker = MappingWalker::new();
    let instance = walker.resolve(&model, EntityId(6)).expect("resolves");
    assert_eq!(instance.mapping_origin, EntityId(2));
    assert_eq!(instance.mapping_target, EntityId(5));
    assert_eq!(instance.mapped_representation, EntityId(3));
}

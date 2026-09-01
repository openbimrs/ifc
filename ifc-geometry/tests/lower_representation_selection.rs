#![cfg(feature = "lowering")]

use axiolid_curve::Curve3;
use axiolid_model::GeometryNode;
use ifc_geometry::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use ifc_geometry::UnitScale;
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn n(value: f64) -> Value {
    Value::Real(value)
}
fn list(values: Vec<Value>) -> Value {
    Value::List(values)
}
fn entity(name: &str, values: Vec<Value>) -> Entity {
    Entity::new(name, values)
}

#[test]
fn plan_selection_lowers_curve_items_instead_of_the_body_path() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(
            "IFCCARTESIANPOINT",
            vec![list(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        entity("IFCDIRECTION", vec![list(vec![n(0.0), n(0.0), n(1.0)])]),
    );
    model.insert(
        EntityId(3),
        entity("IFCDIRECTION", vec![list(vec![n(1.0), n(0.0), n(0.0)])]),
    );
    model.insert(
        EntityId(4),
        entity("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    model.insert(
        EntityId(5),
        entity("IFCELLIPSE", vec![r(4), n(2.0), n(1.0)]),
    );
    model.insert(
        EntityId(6),
        entity("IFCBOUNDINGBOX", vec![r(1), n(10.0), n(10.0), n(10.0)]),
    );
    model.insert(
        EntityId(10),
        entity(
            "IFCSHAPEREPRESENTATION",
            vec![
                Value::Null,
                Value::Text("Plan".into()),
                Value::Text("Curve3D".into()),
                list(vec![r(5)]),
            ],
        ),
    );
    model.insert(
        EntityId(11),
        entity(
            "IFCSHAPEREPRESENTATION",
            vec![
                Value::Null,
                Value::Text("Body".into()),
                Value::Text("BoundingBox".into()),
                list(vec![r(6)]),
            ],
        ),
    );
    model.insert(
        EntityId(12),
        entity(
            "IFCPRODUCTDEFINITIONSHAPE",
            vec![Value::Null, Value::Null, list(vec![r(11), r(10)])],
        ),
    );
    model.insert(
        EntityId(13),
        entity(
            "IFCANNOTATION",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                r(12),
            ],
        ),
    );

    let units = UnitScale {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    let mut session = LoweringSession::new(&model, &units);
    let root =
        lower_product_representation(&mut session, EntityId(13), RepresentationPurpose::Plan)
            .expect("selection resolves")
            .expect("plan exists");
    let lowered = session.finish(root).expect("graph");

    assert!(matches!(
        lowered.graph.get(lowered.root),
        Some(GeometryNode::Curve3(Curve3::Ellipse(_)))
    ));
}

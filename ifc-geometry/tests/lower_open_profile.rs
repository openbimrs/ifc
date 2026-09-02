#![cfg(feature = "lowering")]
use axiolid_curve::Curve2;
use axiolid_model::GeometryNode;
use ifc_geometry::lower::{lower_open_profile_node, lower_profile, LoweringSession};
use ifc_geometry::UnitScale;
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn n(value: f64) -> Value {
    Value::Real(value)
}

fn model(points: &[u64]) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![Value::List(vec![n(0.0), n(0.0)])]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCCARTESIANPOINT", vec![Value::List(vec![n(2.0), n(1.0)])]),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCPOLYLINE",
            vec![Value::List(points.iter().copied().map(r).collect())],
        ),
    );
    m.insert(
        EntityId(4),
        Entity::new(
            "IFCARBITRARYOPENPROFILEDEF",
            vec![Value::Enum("CURVE".into()), Value::Null, r(3)],
        ),
    );
    m
}

#[test]
fn arbitrary_open_profile_preserves_its_exact_open_path() {
    let model = model(&[1, 2]);
    let units = UnitScale::default();
    let mut session = LoweringSession::new(&model, &units);
    let root = lower_open_profile_node(&mut session, EntityId(4)).expect("open profile lowers");
    let lowered = session.finish(root).expect("valid graph");
    let GeometryNode::OpenProfile(profile) = lowered.graph.get(root).expect("root") else {
        panic!("expected OpenProfile")
    };
    let GeometryNode::Curve2(Curve2::Polyline(path)) =
        lowered.graph.get(profile.path).expect("path")
    else {
        panic!("expected exact 2D polyline")
    };
    assert!(!path.closed);
    assert_eq!(path.points.len(), 2);
    assert_eq!(lowered.provenance.source(root), Some(EntityId(4)));
    assert_eq!(lowered.provenance.source(profile.path), Some(EntityId(3)));
}

#[test]
fn area_profile_api_refuses_open_semantics() {
    let model = model(&[1, 2]);
    let error = lower_profile(&model, EntityId(4), &UnitScale::default())
        .expect_err("open path has no area");
    assert!(error.to_string().contains("use lower_open_profile_node"));
}

#[test]
fn geometrically_closed_path_fails_closed() {
    let model = model(&[1, 2, 1]);
    let units = UnitScale::default();
    let mut session = LoweringSession::new(&model, &units);
    let error = lower_open_profile_node(&mut session, EntityId(4))
        .expect_err("closed endpoints are not an open profile");
    assert!(error.to_string().contains("geometrically closed"));
}

#[test]
fn tiny_authored_gap_remains_open_without_an_invented_tolerance() {
    let mut model = model(&[1, 2]);
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(1e-13), n(0.0)])],
        ),
    );
    let units = UnitScale::default();
    let mut session = LoweringSession::new(&model, &units);
    let root = lower_open_profile_node(&mut session, EntityId(4))
        .expect("distinct authored endpoints remain open");
    session.finish(root).expect("open profile graph validates");
}

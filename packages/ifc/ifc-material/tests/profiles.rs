use ifc_material::{
    MaterialUsageDefinition, MaterialView, ResolvedMaterialSelect, StandardCardinalPoint,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(v: &str) -> Value {
    Value::Text(v.into())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

#[test]
fn reads_profiles_sets_offsets_and_usage() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCMATERIAL", vec![text("Steel")]));
    model.insert(EntityId(9), Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALPROFILE",
            vec![
                text("Flange"),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(9)),
                Value::Integer(90),
                text("Primary"),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALPROFILEWITHOFFSETS",
            vec![
                text("Web"),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(9)),
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Real(-0.1), Value::Real(0.1)]),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCMATERIALPROFILESET",
            vec![text("I section"), Value::Null, refs(&[2, 3]), Value::Null],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGE",
            vec![
                Value::Ref(EntityId(4)),
                Value::Integer(15),
                Value::Real(2.0),
            ],
        ),
    );
    model.insert(
        EntityId(6),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            vec![
                Value::Ref(EntityId(4)),
                Value::Integer(5),
                Value::Real(2.0),
                Value::Ref(EntityId(4)),
                Value::Integer(10),
            ],
        ),
    );
    let view = MaterialView::new(&model);
    assert_eq!(
        view.profiles().next().unwrap().profile_id().unwrap(),
        EntityId(9)
    );
    assert_eq!(
        view.profiles_with_offsets()
            .next()
            .unwrap()
            .offset_values()
            .unwrap(),
        [-0.1, 0.1]
    );
    assert_eq!(
        view.profile_sets().next().unwrap().profile_ids().unwrap(),
        vec![EntityId(2), EntityId(3)]
    );
    let usage = view.profile_set_usages().next().unwrap();
    assert_eq!(
        usage.cardinal_point().unwrap().unwrap().standard(),
        Some(StandardCardinalPoint::ShearCenter)
    );
    assert!(matches!(
        view.resolve_material_select(usage.id()).unwrap(),
        ResolvedMaterialSelect::Usage(MaterialUsageDefinition::ProfileSet(_))
    ));
    let tapering = view.tapering_profile_set_usages().next().unwrap();
    assert_eq!(tapering.end_profile_set_id().unwrap(), EntityId(4));
    assert_eq!(
        tapering.cardinal_end_point().unwrap().unwrap().standard(),
        Some(StandardCardinalPoint::GeometricCentroid)
    );
}

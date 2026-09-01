#![cfg(all(feature = "material", feature = "geometry-select"))]

use ifc::geometry::MaterialProfileGeometry;
use ifc::material::MaterialView;
use ifc::{Entity, EntityId, Model, Value};

#[test]
fn material_semantics_and_geometry_join_on_the_same_entity_id() {
    let profile_id = EntityId(7);
    let material_profile_id = EntityId(11);
    let mut model = Model::new();
    model.insert(profile_id, Entity::new("IFCRECTANGLEPROFILEDEF", vec![]));
    model.insert(
        material_profile_id,
        Entity::new(
            "IFCMATERIALPROFILEWITHOFFSETS",
            vec![
                Value::Text("Taper start".into()),
                Value::Text("Semantic and geometric projection".into()),
                Value::Null,
                Value::Ref(profile_id),
                Value::Integer(80),
                Value::Text("STEEL".into()),
                Value::List(vec![Value::Real(-0.01), Value::Real(0.02)]),
            ],
        ),
    );

    let semantic = MaterialView::new(&model)
        .profiles_with_offsets()
        .next()
        .expect("material semantic projection");
    let geometry = MaterialProfileGeometry::new(
        semantic.id(),
        model
            .get(semantic.id())
            .expect("both projections borrow the same model record"),
    )
    .expect("geometry-input projection");

    assert_eq!(semantic.id(), material_profile_id);
    assert_eq!(semantic.profile_id().unwrap(), profile_id);
    assert_eq!(geometry.profile_id().unwrap(), profile_id);
    assert_eq!(semantic.offset_values().unwrap(), [-0.01, 0.02]);
    assert_eq!(geometry.offset_values().unwrap(), Some([-0.01, 0.02]));
    assert_eq!(model.len(), 2, "the join must not duplicate IFC storage");
}

use ifc_material::{MaterialDefinition, MaterialView, ResolvedMaterialSelect};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

#[test]
fn reads_material_identity_constituents_and_sets() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIAL",
            vec![text("Steel"), text("S355"), text("Steel")],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALCONSTITUENT",
            vec![
                text("Core"),
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(0.75),
                text("Structure"),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![text("Composite"), text("desc"), refs(&[2])],
        ),
    );
    let view = MaterialView::new(&model);
    let material = view.materials().next().unwrap();
    assert_eq!(material.name().unwrap(), "Steel");
    assert_eq!(material.category().unwrap(), Some("Steel"));
    let constituent = view.constituents().next().unwrap();
    assert_eq!(constituent.material_id().unwrap(), EntityId(1));
    assert_eq!(constituent.fraction().unwrap(), Some(0.75));
    let set = view.constituent_sets().next().unwrap();
    assert_eq!(set.constituent_ids().unwrap(), Some(vec![EntityId(2)]));
}

#[test]
fn reads_material_properties_relationships_and_lists() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Wood"), Value::Null, text("Wood")]),
    );
    model.insert(EntityId(8), Entity::new("IFCPROPERTYSINGLEVALUE", vec![]));
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALPROPERTIES",
            vec![
                text("Pset_MaterialWood"),
                text("Wood data"),
                refs(&[8]),
                Value::Ref(EntityId(1)),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new("IFCMATERIALLIST", vec![refs(&[1])]),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCMATERIALRELATIONSHIP",
            vec![
                text("Coating"),
                Value::Null,
                Value::Ref(EntityId(1)),
                refs(&[1]),
                text("applied"),
            ],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
            vec![refs(&[9]), Value::Ref(EntityId(1))],
        ),
    );
    let view = MaterialView::new(&model);
    let properties = view.properties_for(EntityId(1)).next().unwrap().unwrap();
    assert_eq!(properties.name().unwrap(), Some("Pset_MaterialWood"));
    assert_eq!(properties.property_ids().unwrap(), vec![EntityId(8)]);
    assert_eq!(
        view.material_lists()
            .next()
            .unwrap()
            .material_ids()
            .unwrap(),
        vec![EntityId(1)]
    );
    assert_eq!(
        view.material_relationships()
            .next()
            .unwrap()
            .expression()
            .unwrap(),
        Some("applied")
    );
    assert_eq!(
        view.classification_relationships()
            .next()
            .unwrap()
            .classification_ids()
            .unwrap(),
        vec![EntityId(9)]
    );
    assert!(matches!(
        view.resolve_material_select(EntityId(1)).unwrap(),
        ResolvedMaterialSelect::Definition(MaterialDefinition::Material(_))
    ));
}

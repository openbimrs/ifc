use ifc_material::{AssignmentSource, MaterialDefinition, MaterialView, ResolvedMaterialSelect};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(v: &str) -> Value {
    Value::Text(v.into())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}
fn relation(kind: &str, related: &[u64], target: u64) -> Entity {
    Entity::new(
        kind,
        vec![
            text("gid"),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(related),
            Value::Ref(EntityId(target)),
        ],
    )
}

#[test]
fn occurrence_assignment_overrides_type_and_type_is_fallback() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Concrete")]),
    );
    model.insert(EntityId(2), Entity::new("IFCMATERIAL", vec![text("Wood")]));
    model.insert(EntityId(10), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(12), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(11), Entity::new("IFCWALLTYPE", vec![]));
    model.insert(EntityId(20), relation("IFCRELASSOCIATESMATERIAL", &[11], 2));
    model.insert(EntityId(21), relation("IFCRELDEFINESBYTYPE", &[10, 12], 11));
    model.insert(EntityId(22), relation("IFCRELASSOCIATESMATERIAL", &[10], 1));
    let view = MaterialView::new(&model);
    let direct = view.assigned_material(EntityId(10)).unwrap().unwrap();
    assert_eq!(direct.source, AssignmentSource::Occurrence);
    let ResolvedMaterialSelect::Definition(MaterialDefinition::Material(material)) =
        direct.material
    else {
        panic!("expected material")
    };
    assert_eq!(material.name().unwrap(), "Concrete");
    let inherited = view.assigned_material(EntityId(12)).unwrap().unwrap();
    assert_eq!(inherited.source, AssignmentSource::Type(EntityId(11)));
    let ResolvedMaterialSelect::Definition(MaterialDefinition::Material(material)) =
        inherited.material
    else {
        panic!("expected material")
    };
    assert_eq!(material.name().unwrap(), "Wood");
}

#[test]
fn mixed_case_ifcxml_type_names_resolve_like_step_names() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IfcMaterial", vec![text("Wood")]));
    model.insert(EntityId(10), Entity::new("IfcWall", vec![]));
    model.insert(EntityId(11), Entity::new("IfcWallType", vec![]));
    model.insert(EntityId(20), relation("IFCRELDEFINESBYTYPE", &[10], 11));
    model.insert(EntityId(21), relation("IFCRELASSOCIATESMATERIAL", &[11], 1));

    let view = MaterialView::new(&model);
    assert!(matches!(
        view.resolve_material_select(EntityId(1)),
        Ok(ResolvedMaterialSelect::Definition(
            MaterialDefinition::Material(_)
        ))
    ));
    let inherited = view.assigned_material(EntityId(10)).unwrap().unwrap();
    assert_eq!(inherited.source, AssignmentSource::Type(EntityId(11)));
}

use ifc_material::{MaterialError, MaterialView};
use ifc_model::{Entity, EntityId, Model, Value};

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

#[test]
fn malformed_enums_cardinals_and_selects_are_explicit_errors() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIALLAYERSET", vec![refs(&[9])]),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALLAYERSETUSAGE",
            vec![
                Value::Ref(EntityId(1)),
                Value::Enum("SIDEWAYS".into()),
                Value::Enum("POSITIVE".into()),
                Value::Real(0.0),
                Value::Null,
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGE",
            vec![Value::Ref(EntityId(8)), Value::Integer(0), Value::Null],
        ),
    );
    model.insert(EntityId(4), Entity::new("IFCWALL", vec![]));
    let view = MaterialView::new(&model);
    assert!(matches!(
        view.layer_set_usages()
            .next()
            .unwrap()
            .layer_set_direction(),
        Err(MaterialError::InvalidValue { .. })
    ));
    assert!(matches!(
        view.profile_set_usages().next().unwrap().cardinal_point(),
        Err(MaterialError::InvalidValue { .. })
    ));
    assert!(matches!(
        view.resolve_material_select(EntityId(4)),
        Err(MaterialError::WrongEntityType { .. })
    ));
}

#[test]
fn duplicate_occurrence_assignments_are_not_guessed() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCMATERIAL", vec![]));
    model.insert(EntityId(10), Entity::new("IFCWALL", vec![]));
    for id in [2, 3] {
        model.insert(
            EntityId(id),
            Entity::new(
                "IFCRELASSOCIATESMATERIAL",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    refs(&[10]),
                    Value::Ref(EntityId(1)),
                ],
            ),
        );
    }
    let error = MaterialView::new(&model)
        .assigned_material(EntityId(10))
        .unwrap_err();
    assert!(matches!(
        error,
        MaterialError::AmbiguousAssignment { count: 2, .. }
    ));
}

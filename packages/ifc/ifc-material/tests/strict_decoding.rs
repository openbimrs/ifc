use ifc_material::{MaterialError, MaterialView};
use ifc_model::{Entity, EntityId, Model, Value};

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

fn assignment(material: u64, objects: Value) -> Entity {
    Entity::new(
        "IFCRELASSOCIATESMATERIAL",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            objects,
            Value::Ref(EntityId(material)),
        ],
    )
}

fn type_relation(type_id: u64, objects: Value) -> Entity {
    Entity::new(
        "IFCRELDEFINESBYTYPE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            objects,
            Value::Ref(EntityId(type_id)),
        ],
    )
}

#[test]
fn rejects_scalar_and_nested_schema_aggregates() {
    for malformed in [
        Value::Ref(EntityId(2)),
        Value::List(vec![Value::List(vec![Value::Ref(EntityId(2))])]),
        Value::List(vec![]),
    ] {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new("IFCMATERIALLAYERSET", vec![malformed]),
        );
        model.insert(
            EntityId(2),
            Entity::new("IFCMATERIALLAYER", vec![Value::Null, Value::Real(0.2)]),
        );
        let view = MaterialView::new(&model);
        let set = view.layer_sets().next().unwrap();
        assert!(matches!(
            view.total_thickness(set),
            Err(MaterialError::InvalidValue { .. }) | Err(MaterialError::MissingAttribute { .. })
        ));
    }
}

#[test]
fn rejects_missing_required_offsets_and_non_finite_total() {
    let mut missing = Model::new();
    missing.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIALLAYERWITHOFFSETS",
            vec![
                Value::Null,
                Value::Real(0.2),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    let layer = MaterialView::new(&missing)
        .layers_with_offsets()
        .next()
        .unwrap();
    assert!(layer.offset_direction().is_err());
    assert!(layer.offset_values().is_err());

    let mut overflow = Model::new();
    overflow.insert(
        EntityId(1),
        Entity::new("IFCMATERIALLAYERSET", vec![refs(&[2, 3])]),
    );
    for id in [2, 3] {
        overflow.insert(
            EntityId(id),
            Entity::new("IFCMATERIALLAYER", vec![Value::Null, Value::Real(f64::MAX)]),
        );
    }
    let view = MaterialView::new(&overflow);
    let set = view.layer_sets().next().unwrap();
    assert!(matches!(
        view.total_thickness(set),
        Err(MaterialError::InvalidValue { .. })
    ));
}

#[test]
fn assignment_resolution_rejects_missing_objects_and_malformed_relations() {
    let empty = Model::new();
    assert!(MaterialView::new(&empty)
        .assigned_material(EntityId(999))
        .is_err());

    let mut malformed = Model::new();
    malformed.insert(EntityId(1), Entity::new("IFCWALL", vec![]));
    malformed.insert(EntityId(2), Entity::new("IFCWALLTYPE", vec![]));
    malformed.insert(
        EntityId(3),
        Entity::new("IFCMATERIAL", vec![Value::Text("x".into())]),
    );
    malformed.insert(EntityId(4), type_relation(2, Value::Ref(EntityId(1))));
    malformed.insert(EntityId(5), assignment(3, refs(&[2])));
    assert!(MaterialView::new(&malformed)
        .assigned_material(EntityId(1))
        .is_err());
}

#[test]
fn duplicate_type_relations_are_ambiguous_even_for_the_same_type() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(2), Entity::new("IFCWALLTYPE", vec![]));
    model.insert(
        EntityId(3),
        Entity::new("IFCMATERIAL", vec![Value::Text("x".into())]),
    );
    for id in [4, 5] {
        model.insert(EntityId(id), type_relation(2, refs(&[1])));
    }
    model.insert(EntityId(6), assignment(3, refs(&[2])));
    assert!(matches!(
        MaterialView::new(&model).assigned_material(EntityId(1)),
        Err(MaterialError::AmbiguousType { .. })
    ));
}

#[test]
fn dangling_or_non_type_relating_type_is_rejected() {
    for target in [99, 3] {
        let mut model = Model::new();
        model.insert(EntityId(1), Entity::new("IFCWALL", vec![]));
        model.insert(
            EntityId(3),
            Entity::new("IFCMATERIAL", vec![Value::Text("x".into())]),
        );
        model.insert(EntityId(4), type_relation(target, refs(&[1])));
        assert!(MaterialView::new(&model)
            .assigned_material(EntityId(1))
            .is_err());
    }
}

#[test]
fn malformed_optional_values_and_where_rules_are_errors() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIALLAYER",
            vec![
                Value::Null,
                Value::Real(0.1),
                Value::Text("not logical".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Integer(101),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGE",
            vec![Value::Ref(EntityId(8)), Value::Null, Value::Real(0.0)],
        ),
    );
    let view = MaterialView::new(&model);
    let layer = view.layers().next().unwrap();
    assert!(matches!(
        layer.is_ventilated(),
        Err(MaterialError::InvalidValue { .. })
    ));
    assert!(matches!(
        layer.priority(),
        Err(MaterialError::InvalidValue { .. })
    ));
    let usage = view.profile_set_usages().next().unwrap();
    assert!(matches!(
        usage.reference_extent(),
        Err(MaterialError::InvalidValue { .. })
    ));
}

#[test]
fn typed_wrapper_nesting_is_bounded() {
    let mut value = Value::Text("Steel".into());
    for _ in 0..9 {
        value = Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(value),
        };
    }
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IFCMATERIAL", vec![value]));
    let material = MaterialView::new(&model).materials().next().unwrap();
    assert!(matches!(
        material.name(),
        Err(MaterialError::InvalidValue { .. })
    ));
}

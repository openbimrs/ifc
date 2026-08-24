use ifc_material::{DirectionSense, LayerSetDirection, LogicalValue, MaterialView};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn enum_value(value: &str) -> Value {
    Value::Enum(value.into())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}

#[test]
fn reads_layers_offsets_sets_and_total_thickness() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Concrete")]),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALLAYER",
            vec![
                Value::Ref(EntityId(1)),
                Value::Real(0.2),
                Value::Bool(false),
                text("Core"),
                Value::Null,
                text("Structure"),
                Value::Integer(80),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCMATERIALLAYERWITHOFFSETS",
            vec![
                Value::Ref(EntityId(1)),
                Value::Real(0.06),
                Value::LogicalUnknown,
                text("Finish"),
                Value::Null,
                Value::Null,
                Value::Null,
                enum_value("AXIS2"),
                Value::List(vec![Value::Real(-0.01), Value::Real(0.02)]),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCMATERIALLAYERSET",
            vec![refs(&[2, 3]), text("Wall"), Value::Null],
        ),
    );
    let view = MaterialView::new(&model);
    let layer = view.layers().next().unwrap();
    assert_eq!(layer.thickness().unwrap(), 0.2);
    assert_eq!(layer.is_ventilated().unwrap(), Some(LogicalValue::False));
    let offset = view.layers_with_offsets().next().unwrap();
    assert_eq!(offset.offset_direction().unwrap(), LayerSetDirection::Axis2);
    assert_eq!(offset.offset_values().unwrap(), [-0.01, 0.02]);
    assert_eq!(
        view.total_thickness(view.layer_sets().next().unwrap())
            .unwrap(),
        0.26
    );
}

#[test]
fn reads_layer_set_usage_enums_and_dimensions() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIALLAYERSET",
            vec![refs(&[8]), text("Wall"), Value::Null],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCMATERIALLAYERSETUSAGE",
            vec![
                Value::Ref(EntityId(1)),
                enum_value("AXIS3"),
                enum_value("NEGATIVE"),
                Value::Real(-0.1),
                Value::Real(3.0),
            ],
        ),
    );
    let view = MaterialView::new(&model);
    let usage = view.layer_set_usages().next().unwrap();
    assert_eq!(usage.layer_set_id().unwrap(), EntityId(1));
    assert_eq!(
        usage.layer_set_direction().unwrap(),
        LayerSetDirection::Axis3
    );
    assert_eq!(usage.direction_sense().unwrap(), DirectionSense::Negative);
    assert_eq!(usage.offset_from_reference_line().unwrap(), -0.1);
    assert_eq!(usage.reference_extent().unwrap(), Some(3.0));
}

#![allow(dead_code)]

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

pub const GUID: &str = "2O2Fr$t4X7Zf8NOew3FLOH";

pub fn named(schema: &Schema, type_name: &str, fields: &[(&str, Value)]) -> Entity {
    let names = schema.attribute_names(type_name);
    assert!(
        !names.is_empty(),
        "{type_name} must exist in selected schema"
    );
    let mut values = vec![Value::Null; names.len()];
    for (name, value) in fields {
        let slot = names
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name}.{name} missing from selected schema"));
        values[slot] = value.clone();
    }
    Entity::new(type_name.to_ascii_uppercase(), values)
}

pub fn model(schema_token: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema_token.to_owned()];
    model
}

pub fn text(value: &str) -> Value {
    Value::Text(value.into())
}

pub fn enumeration(value: &str) -> Value {
    Value::Enum(value.into())
}

pub fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

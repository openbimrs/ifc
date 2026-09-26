//! Building blocks for the adversarial fixtures.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;
use ifc_validate::Report;

/// A well-formed GlobalId: 22 characters, as `STRING(22) FIXED` demands.
pub const GUID_A: &str = "0hMOPMBpTAoOL$IPqA1$xY";
/// A second, distinct well-formed GlobalId.
pub const GUID_B: &str = "1sEzC8v31DshmvW5t5P631";

/// A record of `type_name` at its full declared width, every slot `$`
/// except the named ones.
///
/// Built from the schema tables so a fixture never guesses a slot index,
/// and so the record never trips the slot-count guard by accident.
pub fn entity(schema: &Schema, type_name: &str, values: &[(&str, Value)]) -> Entity {
    let names = schema.attribute_names(type_name);
    assert!(
        schema.entity(type_name).is_some(),
        "{type_name} is not in {}",
        schema.name()
    );
    let mut attributes = vec![Value::Null; names.len()];
    for (name, value) in values {
        let index = names
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name} has no {name} in {}", schema.name()));
        attributes[index] = value.clone();
    }
    Entity::new(type_name, attributes)
}

/// A model whose header declares `token`, so header rules are quiet.
pub fn declared(token: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![token.to_string()];
    model.header_mut().implementation_level = "2;1".to_string();
    model
}

/// Validates against the bundled IFC4 tables.
pub fn ifc4(model: &Model) -> Report {
    ifc_validate::validate(model, ifc_schema::ifc4())
}

/// A typed wrapper, as `IFCLABEL('x')` is written.
pub fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

/// A text value.
pub fn text(value: &str) -> Value {
    Value::Text(value.into())
}

/// An `IfcWall` with a GlobalId, and one attribute overridden.
pub fn wall(schema: &Schema, guid: &str, extra: &[(&str, Value)]) -> Entity {
    let mut values = vec![("GlobalId", text(guid))];
    values.extend(extra.iter().cloned());
    entity(schema, "IFCWALL", &values)
}

/// One IFC4 model holding a single `IfcWall` whose `attribute` is `value`.
pub fn wall_with(attribute: &str, value: Value) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(wall(schema, GUID_A, &[(attribute, value)]));
    ifc4(&model)
}

/// A relation of `relation` whose relating end is `relating` and whose
/// related objects are `related`, over two walls `#1` and `#2`.
pub fn relation(
    schema: &Schema,
    relation: &str,
    relating_attribute: &str,
    relating: u64,
    related: u64,
) -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), wall(schema, GUID_A, &[]));
    model.insert(EntityId(2), wall(schema, GUID_B, &[]));
    model.push(entity(
        schema,
        relation,
        &[
            (relating_attribute, Value::Ref(EntityId(relating))),
            (
                "RelatedObjects",
                Value::List(vec![Value::Ref(EntityId(related))]),
            ),
        ],
    ));
    model
}

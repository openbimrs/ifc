//! Test fixtures for building throwaway entities and models.
//!
//! # Why a shared kit
//!
//! Every view test needs an `Entity` with attributes at exact STEP slots. Hand
//! rolling `Value` trees per test buries the property being verified under
//! construction noise, and a typo in the construction silently tests the wrong
//! slot. Building fixtures through one helper keeps the assertion visible.

use ifc_model::{Entity, EntityId, Model, Value};

/// An entity with the given upper-cased type name and positional attributes.
pub fn entity(type_name: &str, attributes: Vec<Value>) -> Entity {
    Entity::new(type_name, attributes)
}

/// A reference value to `#id`.
pub fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

/// A real-number value.
pub fn n(value: f64) -> Value {
    Value::Real(value)
}

/// An integer value, as STEP writes `IfcPositiveInteger`.
pub fn i(value: i64) -> Value {
    Value::Integer(value)
}

/// An enumeration value, written `.TOKEN.` in a file.
pub fn e(token: &str) -> Value {
    Value::Enum(token.into())
}

/// A list value.
pub fn list(items: Vec<Value>) -> Value {
    Value::List(items)
}

/// A list of references, the common `SET OF Ifc...` shape.
pub fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| r(*id)).collect())
}

/// A list of integers, the common index-list shape.
pub fn ints(values: &[i64]) -> Value {
    Value::List(values.iter().map(|v| i(*v)).collect())
}

/// A list of integer lists, e.g. `CoordIndex`.
pub fn int_grid(rows: &[&[i64]]) -> Value {
    Value::List(rows.iter().map(|row| ints(row)).collect())
}

/// A model holding the given `(id, entity)` pairs.
pub fn model(entities: Vec<(u64, Entity)>) -> Model {
    let mut m = Model::new();
    for (id, e) in entities {
        m.insert(EntityId(id), e);
    }
    m
}

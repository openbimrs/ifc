//! Staging authored quantity updates onto a transaction.
//!
//! # This module stages; it does not commit
//!
//! Every function here takes a `&mut Transaction` and adds edits to it. None
//! of them touch the model. That is deliberate: a takeoff run updates areas on
//! two hundred elements, and those either all land or none do. If each helper
//! committed, a failure halfway through would leave a file whose quantities
//! disagree with each other, which is worse than a file that was never
//! updated.
//!
//! The caller decides the boundary:
//!
//! ```
//! use ifc_model::{Model, Transaction};
//! use ifc_properties::{set_quantity_value, QuantityKind};
//!
//! # fn run(model: &mut Model, area: ifc_model::EntityId) -> Result<(), Box<dyn std::error::Error>> {
//! let mut tx = Transaction::new(model);
//! set_quantity_value(&mut tx, model, area, 12.5)?;
//! // ... stage the rest of the takeoff ...
//! tx.commit(model).map_err(|c| format!("{c:?}"))?;
//! # Ok(())
//! # }
//! ```
//!
//! # Why writes are refused rather than coerced
//!
//! A quantity's value slot is typed: `IfcQuantityArea` holds an
//! `IfcAreaMeasure`. Writing a bare real into it would produce a file that
//! parses and has lost the statement of what the number means -- the exact
//! failure `value.rs` exists to prevent on read. So these helpers preserve
//! the declared measure type, and refuse when the target is not the kind of
//! quantity the caller thinks it is.

use ifc_model::{EntityId, Model, Transaction, Value};

use crate::error::PropertyError;
use crate::quantity::set::QuantityKind;

/// Slot of the value on every `IfcPhysicalSimpleQuantity` subtype.
///
/// Verified against IFC4 EXPRESS: `IfcPhysicalQuantity` contributes `Name`
/// and `Description`, `IfcPhysicalSimpleQuantity` adds `Unit`, and the
/// concrete subtype's own measure follows.
const SIMPLE_VALUE_SLOT: usize = 3;

/// Stage a new value for an existing simple quantity.
///
/// The measure type already on the entity is preserved, so an
/// `IfcQuantityArea` keeps writing `IfcAreaMeasure`. A quantity whose value
/// slot is empty or untyped is written with the measure implied by its own
/// entity type, which is the only defensible reading: the schema fixes which
/// measure each subtype carries.
///
/// # Errors
///
/// [`PropertyError::NotAQuantity`] if `id` is not a simple quantity, and
/// [`PropertyError::MissingEntity`] if it is not in the model.
pub fn set_quantity_value(
    tx: &mut Transaction,
    model: &Model,
    id: EntityId,
    value: f64,
) -> Result<(), PropertyError> {
    let entity = model.get(id).ok_or(PropertyError::MissingEntity { id })?;
    let kind = QuantityKind::from_type_name(&entity.type_name).ok_or_else(|| {
        PropertyError::NotAQuantity {
            id,
            type_name: entity.type_name.to_string(),
        }
    })?;
    // Prefer the measure already written; fall back to the one the entity
    // type implies. Never write a bare number.
    let measure = entity
        .attribute(SIMPLE_VALUE_SLOT)
        .and_then(declared_measure)
        .unwrap_or_else(|| kind.measure_type().to_string());

    let numeric = if kind == QuantityKind::Count {
        // IfcCountMeasure is an INTEGER in IFC4; writing 3.0 into it produces
        // a file that is wrong against the schema even though it parses.
        Value::Integer(value as i64)
    } else {
        Value::Real(value)
    };

    tx.set_attribute(
        id,
        SIMPLE_VALUE_SLOT,
        Value::Typed {
            type_name: measure.into(),
            value: Box::new(numeric),
        },
    );
    Ok(())
}

/// Stage a new `Name` for a quantity or property.
///
/// Slot 0 on both `IfcPhysicalQuantity` and `IfcProperty`.
///
/// # Errors
///
/// [`PropertyError::MissingEntity`] if `id` is not in the model.
pub fn set_name(
    tx: &mut Transaction,
    model: &Model,
    id: EntityId,
    name: &str,
) -> Result<(), PropertyError> {
    if model.get(id).is_none() {
        return Err(PropertyError::MissingEntity { id });
    }
    tx.set_attribute(id, 0, Value::Text(name.into()));
    Ok(())
}

/// Stage a new `Description`, or clear it with `None`.
///
/// Slot 1 on both `IfcPhysicalQuantity` and `IfcProperty`. `None` writes
/// `Value::Null`, which is STEP's `$` -- the attribute is genuinely unset,
/// not set to an empty string. The distinction survives a round trip and a
/// consumer can tell "no description" from "description is blank".
///
/// # Errors
///
/// [`PropertyError::MissingEntity`] if `id` is not in the model.
pub fn set_description(
    tx: &mut Transaction,
    model: &Model,
    id: EntityId,
    description: Option<&str>,
) -> Result<(), PropertyError> {
    if model.get(id).is_none() {
        return Err(PropertyError::MissingEntity { id });
    }
    let value = match description {
        Some(text) => Value::Text(text.into()),
        None => Value::Null,
    };
    tx.set_attribute(id, 1, value);
    Ok(())
}

/// Stage a brand-new simple quantity, returning the id reserved for it.
///
/// The entity is created with the measure type its kind implies and no unit,
/// meaning "the project default applies" -- which is what most authored
/// quantities mean. Attach it to a set with [`add_quantity_to_set`].
pub fn create_quantity(
    tx: &mut Transaction,
    kind: QuantityKind,
    name: &str,
    value: f64,
) -> EntityId {
    let numeric = if kind == QuantityKind::Count {
        Value::Integer(value as i64)
    } else {
        Value::Real(value)
    };
    tx.create(ifc_model::Entity::new(
        kind.type_name(),
        vec![
            Value::Text(name.into()),
            Value::Null,
            Value::Null,
            Value::Typed {
                type_name: kind.measure_type().into(),
                value: Box::new(numeric),
            },
        ],
    ))
}

/// Stage adding a quantity to an existing `IfcElementQuantity`.
///
/// Reads the set's current contents and writes back the extended list, so
/// this is a read-modify-write against the model as it stands. Two calls
/// adding to the same set in one transaction would each be computed from the
/// same starting list and the second would win -- add both in one call
/// instead.
///
/// # Errors
///
/// [`PropertyError::MissingEntity`] if the set is absent, and
/// [`PropertyError::NotAQuantitySet`] if it is not an `IfcElementQuantity`.
pub fn add_quantity_to_set(
    tx: &mut Transaction,
    model: &Model,
    set: EntityId,
    quantities: &[EntityId],
) -> Result<(), PropertyError> {
    let entity = model
        .get(set)
        .ok_or(PropertyError::MissingEntity { id: set })?;
    if !entity.type_name.eq_ignore_ascii_case("IFCELEMENTQUANTITY") {
        return Err(PropertyError::NotAQuantitySet {
            id: set,
            type_name: entity.type_name.to_string(),
        });
    }

    /// `Quantities` slot on `IfcElementQuantity`: `IfcRoot` contributes four
    /// attributes, then `MethodOfMeasurement` at 4.
    const QUANTITIES_SLOT: usize = 5;

    let mut members: Vec<Value> = entity
        .attribute(QUANTITIES_SLOT)
        .and_then(Value::as_list)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();
    let existing: Vec<EntityId> = members.iter().filter_map(Value::as_ref_id).collect();
    for id in quantities {
        // A set naming the same quantity twice is malformed; adding one that
        // is already there is a no-op rather than a duplicate.
        if !existing.contains(id) {
            members.push(Value::Ref(*id));
        }
    }
    tx.set_attribute(set, QUANTITIES_SLOT, Value::List(members));
    Ok(())
}

/// The measure type named by a typed value, if it has one.
fn declared_measure(value: &Value) -> Option<String> {
    match value {
        Value::Typed { type_name, .. } => Some(type_name.to_string()),
        _ => None,
    }
}

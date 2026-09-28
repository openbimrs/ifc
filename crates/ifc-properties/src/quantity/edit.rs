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
//! # The value is written bare
//!
//! A quantity's value slot is typed by its declaration: `IfcQuantityArea`
//! declares `AreaValue : IfcAreaMeasure`, a defined type and not a SELECT.
//! The measure is therefore already stated by the schema, and ISO 10303-21
//! writes the simple value there (`12.5`); a typed parameter such as
//! `IFCAREAMEASURE(12.5)` belongs only in a SELECT slot, where the type is
//! otherwise ambiguous (#190). The readers accept both forms, because files
//! in the wild carry both. These helpers refuse when the target is not the
//! kind of quantity the caller thinks it is, rather than coerce it.

use ifc_model::{EntityId, Model, Transaction, Value};

use crate::error::PropertyError;
use crate::quantity::release::bind;
use crate::quantity::set::QuantityKind;

/// Stage a new value for an existing simple quantity.
///
/// The value is written into the slot the model's declared release gives
/// the quantity's value attribute (`release.rs` has the binding), as a bare
/// number: the schema fixes which measure each subtype carries, so no
/// wrapper is written. A value read in the typed form
/// (`IFCAREAMEASURE(12.5)`) is replaced by the bare one. This also repairs
/// a quantity read as [`Quantity::Unresolved`](crate::Quantity::Unresolved).
///
/// # Errors
///
/// Nothing is staged on an error:
/// [`PropertyError::MissingEntity`] if `id` is not in the model,
/// [`PropertyError::NotAQuantity`] if it is not a simple quantity,
/// [`PropertyError::MultipleSchemas`] or [`PropertyError::UnsupportedSchema`]
/// if the model binds no single release,
/// [`PropertyError::EntityNotInSchema`] if that release does not declare the
/// quantity's entity, [`PropertyError::MalformedEntitySlots`] if the record
/// does not have the release's attribute count, and
/// [`PropertyError::AuthoringInvalid`] for a value its measure cannot hold.
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
    let layout = bind(model)?;
    let name = layout.entity(kind)?;
    let expected = layout.arity(&name);
    if entity.attributes.len() != expected {
        return Err(PropertyError::MalformedEntitySlots {
            id,
            type_name: entity.type_name.to_string(),
            expected,
            actual: entity.attributes.len(),
        });
    }
    let slot = layout
        .slot(&name, kind.value_attribute())
        .expect("every release that declares a quantity declares its value");
    tx.set_attribute(id, slot, layout.scalar(kind, value)?);
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

/// Stage a brand-new simple quantity in `model`'s release, returning the id
/// reserved for it.
///
/// The entity is created with its value written bare in the slot whose
/// declared measure its kind implies, and no unit, meaning "the project
/// default applies" -- which is what most authored quantities mean. Attach
/// it to a set with [`add_quantity_to_set`].
///
/// # Errors
///
/// As [`create_quantity_with`].
pub fn create_quantity(
    tx: &mut Transaction,
    model: &Model,
    kind: QuantityKind,
    name: &str,
    value: f64,
) -> Result<EntityId, PropertyError> {
    create_quantity_with(tx, model, kind, name, value, QuantityExtras::default())
}

/// The optional attributes of an `IfcPhysicalSimpleQuantity`.
///
/// Separated from [`create_quantity`] so the common call stays short,
/// while `Description`, `Unit` and `Formula` remain reachable. They are
/// attributes of the entity, not decoration: the reader in this crate
/// resolves all three.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuantityExtras<'a> {
    /// `IfcPhysicalQuantity.Description`.
    pub description: Option<&'a str>,
    /// `IfcPhysicalSimpleQuantity.Unit`, an `IfcNamedUnit` reference.
    ///
    /// Left unset the quantity is read in the project's default unit for
    /// its measure, which is usually what a take-off wants.
    pub unit: Option<EntityId>,
    /// `Formula`, how the quantity was derived. IFC4 and IFC4X3 only: an
    /// IFC2X3 quantity has no `Formula`, and one given for it is refused.
    pub formula: Option<&'a str>,
}

/// Stage a simple quantity with its optional attributes, in `model`'s
/// release.
///
/// # The record has the release's layout
///
/// Attributes are placed by name in the declared release's table. IFC4 and
/// IFC4X3 quantities have five attributes (`Name`, `Description`, `Unit`,
/// the value, `Formula`); IFC2X3 ones have four, with no `Formula`. A
/// model without `FILE_SCHEMA` binds IFC4. The value is written bare
/// (`IFCQUANTITYAREA('A',$,$,12.5,$)`), since its declared type is a
/// defined measure and not a SELECT.
///
/// # Errors
///
/// Nothing is staged on an error:
/// - [`PropertyError::MultipleSchemas`] or
///   [`PropertyError::UnsupportedSchema`]: the model binds no single release.
/// - [`PropertyError::EntityNotInSchema`]: the release does not declare the
///   kind, as `IfcQuantityNumber` outside IFC4X3.
/// - [`PropertyError::AuthoringNotInSchema`]: a `Formula` for an IFC2X3
///   model, which has none.
/// - [`PropertyError::AuthoringInvalid`]: a non-finite value, or a
///   fractional count where `IfcCountMeasure` is `INTEGER` (IFC4X3). In
///   IFC2X3 and IFC4 it is `NUMBER`, so a fractional count is written as a
///   real, and never truncated.
pub fn create_quantity_with(
    tx: &mut Transaction,
    model: &Model,
    kind: QuantityKind,
    name: &str,
    value: f64,
    extras: QuantityExtras<'_>,
) -> Result<EntityId, PropertyError> {
    let layout = bind(model)?;
    let entity = layout.entity(kind)?;
    let numeric = layout.scalar(kind, value)?;
    let text = |text: Option<&str>| text.map_or(Value::Null, |text| Value::Text(text.into()));
    let record = layout.record(
        kind,
        &entity,
        vec![
            ("Name", Value::Text(name.into())),
            ("Description", text(extras.description)),
            ("Unit", extras.unit.map_or(Value::Null, Value::Ref)),
            // Bare: the declared type is a defined measure, not a SELECT.
            (kind.value_attribute(), numeric),
            ("Formula", text(extras.formula)),
        ],
    )?;
    Ok(tx.create(record))
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

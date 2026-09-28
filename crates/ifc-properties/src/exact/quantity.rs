//! Quantity sets in exact resolution (#66).
//!
//! `IfcRelDefinesByProperties` and `IfcTypeObject.HasPropertySets` carry any
//! `IfcPropertySetDefinition`, not only `IfcPropertySet`. buildingSMART IDS
//! treats a quantity like a property, so a named quantity is resolved here
//! exactly rather than reported as absent:
//!
//! ```text
//! IfcElementQuantity         2 = Name   5 = Quantities   (IFC2X3, IFC4, IFC4X3)
//! IfcPhysicalQuantity        0 = Name
//! IfcPhysicalSimpleQuantity  2 = Unit   3 = <Length|Area|...>Value
//! ```
//!
//! Positions are read from the bound release's table, not hard-coded, and
//! the value's type is that attribute's declared measure (`IfcLengthMeasure`
//! for `LengthValue`), because a quantity stores a bare number.
//!
//! A predefined set (`IfcDoorLiningProperties` and the like) holds values in
//! attributes of its own entity; `predefined.rs` reads those.

use ifc_model::{Entity, EntityId, Model, Value};

use super::complex::{complex_value, is_complex};
use super::refs::{nonempty_refs_at, text_at};
use super::release::Release;
use super::value::{exact_value, ResolvedValue};
use super::ExactPropertyError;

/// The slot `attribute` occupies on `entity` in the bound release.
pub(super) fn slot(release: Release, entity: &str, attribute: &str) -> usize {
    release
        .schema
        .attribute_names(entity)
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
        .unwrap_or_else(|| panic!("{entity}.{attribute} is declared in every bundled release"))
}

/// Every member of an `IfcElementQuantity` with its `Name`, in file order.
///
/// # Errors
///
/// A malformed or empty `Quantities`, a missing, foreign or
/// non-`IfcPhysicalQuantity` member, a member with the wrong arity, or one
/// without a text name.
pub(super) fn quantity_members<'m>(
    model: &'m Model,
    release: Release,
    set_id: EntityId,
    set: &Entity,
) -> Result<Vec<(EntityId, &'m str)>, ExactPropertyError> {
    let schema = release.schema;
    let slot = slot(release, "IFCELEMENTQUANTITY", "Quantities");
    let mut members = Vec::new();
    for quantity_id in nonempty_refs_at(set_id, set.attributes.get(slot), "Quantities")? {
        let quantity = model
            .get(quantity_id)
            .ok_or(ExactPropertyError::MissingReference {
                from: set_id,
                to: quantity_id,
            })?;
        if schema.entity(quantity.type_name.as_ref()).is_none() {
            return Err(release.not_in_schema(quantity_id, quantity.type_name.clone()));
        }
        if !schema.is_a(quantity.type_name.as_ref(), "IFCPHYSICALQUANTITY") {
            return Err(ExactPropertyError::UnsupportedProperty {
                entity: quantity_id,
                type_name: quantity.type_name.clone(),
            });
        }
        release.require_exact_slots(quantity_id, quantity)?;
        let name = text_at(quantity_id, quantity.attributes.first(), "Name")?;
        members.push((quantity_id, name));
    }
    Ok(members)
}

/// The value of quantity `quantity_id`: a simple quantity's, or an
/// `IfcPhysicalComplexQuantity` as
/// [`ExactValue::Complex`](super::ExactValue::Complex) (#208).
///
/// # Errors
///
/// A malformed unit or value, or any error of a complex quantity.
pub(super) fn quantity_value(
    model: &Model,
    release: Release,
    quantity_id: EntityId,
) -> Result<ResolvedValue, ExactPropertyError> {
    let quantity = model.get(quantity_id).expect("checked reference");
    if is_complex(release, quantity) {
        return complex_value(model, release, quantity_id, quantity);
    }
    simple_quantity_value(model, release, quantity_id, quantity)
}

/// The value of `IfcPhysicalSimpleQuantity` `quantity`, whose arity the
/// caller confirmed.
///
/// # Errors
///
/// [`ExactPropertyError::UnsupportedProperty`] for any other quantity kind,
/// or a malformed unit or value.
pub(super) fn simple_quantity_value(
    model: &Model,
    release: Release,
    quantity_id: EntityId,
    quantity: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
    if !release
        .schema
        .is_a(quantity.type_name.as_ref(), "IFCPHYSICALSIMPLEQUANTITY")
    {
        return Err(ExactPropertyError::UnsupportedProperty {
            entity: quantity_id,
            type_name: quantity.type_name.clone(),
        });
    }
    simple_value(model, release, quantity_id, quantity)
}

/// The value, its declared measure type, and the unit of a simple quantity.
fn simple_value(
    model: &Model,
    release: Release,
    quantity_id: EntityId,
    quantity: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
    let schema = release.schema;
    let unit_slot = slot(release, "IFCPHYSICALSIMPLEQUANTITY", "Unit");
    let unit_id = match &quantity.attributes[unit_slot] {
        Value::Null => None,
        Value::Ref(unit_id) => {
            let unit = model
                .get(*unit_id)
                .ok_or(ExactPropertyError::MissingReference {
                    from: quantity_id,
                    to: *unit_id,
                })?;
            if schema.entity(unit.type_name.as_ref()).is_none() {
                return Err(release.not_in_schema(*unit_id, unit.type_name.clone()));
            }
            if !schema.is_a(unit.type_name.as_ref(), "IFCNAMEDUNIT") {
                return Err(ExactPropertyError::UnsupportedUnit {
                    property: quantity_id,
                });
            }
            release.require_exact_slots(*unit_id, unit)?;
            Some(*unit_id)
        }
        _ => {
            return Err(ExactPropertyError::UnsupportedUnit {
                property: quantity_id,
            })
        }
    };
    // Every simple quantity declares its value directly after `Unit`.
    let attributes = schema.attributes(quantity.type_name.as_ref());
    let declared = attributes
        .get(unit_slot + 1)
        .map(|attribute| attribute.type_name.to_ascii_uppercase())
        .ok_or(ExactPropertyError::UnsupportedValue {
            property: quantity_id,
        })?;
    let value = match &quantity.attributes[unit_slot + 1] {
        Value::Null => {
            return Err(ExactPropertyError::MissingValueSlot {
                property: quantity_id,
            })
        }
        raw @ (Value::Real(_) | Value::Integer(_)) => exact_value(quantity_id, Some(raw))?,
        Value::Typed { type_name, value }
            if type_name.eq_ignore_ascii_case(&declared)
                && matches!(value.as_ref(), Value::Real(_) | Value::Integer(_)) =>
        {
            exact_value(quantity_id, Some(value))?
        }
        _ => {
            return Err(ExactPropertyError::UnsupportedValue {
                property: quantity_id,
            })
        }
    };
    Ok(ResolvedValue {
        value,
        value_type: Some(declared.into()),
        unit_id,
    })
}

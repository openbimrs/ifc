//! Quantity sets and predefined property sets in exact resolution (#66).
//!
//! `IfcRelDefinesByProperties` and `IfcTypeObject.HasPropertySets` carry any
//! `IfcPropertySetDefinition`, not only `IfcPropertySet`. buildingSMART IDS
//! treats a quantity like a property, so a named quantity is resolved here
//! exactly rather than reported as absent:
//!
//! ```text
//! IfcElementQuantity         2 = Name   5 = Quantities   (IFC2X3 and IFC4)
//! IfcPhysicalQuantity        0 = Name
//! IfcPhysicalSimpleQuantity  2 = Unit   3 = <Length|Area|...>Value
//! ```
//!
//! Positions are read from the bound release's table, not hard-coded, and
//! the value's type is that attribute's declared measure (`IfcLengthMeasure`
//! for `LengthValue`), because a quantity stores a bare number.
//!
//! A predefined set (`IfcDoorLiningProperties` and the like) holds values in
//! entity attributes instead of named properties. Those are not resolved;
//! but when one of its own attributes carries the requested name, the answer
//! is refused rather than claimed absent.

use ifc_model::{Entity, EntityId, Model, Value};

use super::refs::{nonempty_refs_at, text_at};
use super::release::Release;
use super::value::{exact_value, ResolvedValue};
use super::ExactPropertyError;

/// The slot `attribute` occupies on `entity` in the bound release.
fn slot(release: Release, entity: &str, attribute: &str) -> usize {
    release
        .schema
        .attribute_names(entity)
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
        .unwrap_or_else(|| panic!("{entity}.{attribute} is declared in IFC2X3 and IFC4"))
}

/// The quantity named `wanted` in an `IfcElementQuantity`, if any.
///
/// # Errors
///
/// A member that is not an `IfcPhysicalQuantity`, two members with the name,
/// a matching complex quantity, or a malformed unit or value.
pub(super) fn find_quantity(
    model: &Model,
    release: Release,
    set_id: EntityId,
    set: &Entity,
    wanted: &str,
) -> Result<Option<(EntityId, ResolvedValue)>, ExactPropertyError> {
    let schema = release.schema;
    let members = slot(release, "IFCELEMENTQUANTITY", "Quantities");
    let mut matching = None;
    for quantity_id in nonempty_refs_at(set_id, set.attributes.get(members), "Quantities")? {
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
        if text_at(quantity_id, quantity.attributes.first(), "Name")? != wanted {
            continue;
        }
        if let Some(first) = matching.replace(quantity_id) {
            return Err(ExactPropertyError::DuplicateMatchingProperties {
                set: set_id,
                first,
                second: quantity_id,
            });
        }
    }
    let Some(quantity_id) = matching else {
        return Ok(None);
    };
    let quantity = model.get(quantity_id).expect("checked reference");
    if !schema.is_a(quantity.type_name.as_ref(), "IFCPHYSICALSIMPLEQUANTITY") {
        return Err(ExactPropertyError::UnsupportedProperty {
            entity: quantity_id,
            type_name: quantity.type_name.clone(),
        });
    }
    Ok(Some((
        quantity_id,
        simple_value(model, release, quantity_id, quantity)?,
    )))
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

/// Whether a predefined set could hold `wanted`: one of its own attributes
/// (not those every `IfcPropertySetDefinition` inherits) has that name, and
/// its `Name` is `wanted_set`, absent, or not asked for.
pub(super) fn predefined_may_hold(
    release: Release,
    set: &Entity,
    wanted_set: Option<&str>,
    wanted: &str,
) -> bool {
    let schema = release.schema;
    let inherited = schema.attribute_names("IFCPROPERTYSETDEFINITION").len();
    let own = schema.attribute_names(set.type_name.as_ref());
    if !own.iter().skip(inherited).any(|name| *name == wanted) {
        return false;
    }
    match (
        wanted_set,
        set.attributes
            .get(2)
            .and_then(|v| v.unwrap_typed().as_text()),
    ) {
        (Some(asked), Some(name)) => asked == name,
        _ => true,
    }
}

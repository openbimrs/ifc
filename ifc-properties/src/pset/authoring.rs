//! Transactional authoring of property sets and their attachment.
//!
//! Reading lives in the sibling modules; this is the write side.
//! Slot order is resolved from the bundled IFC4X3 schema, not
//! from memory: a pset writes HasProperties at slot 4 because
//! four IfcRoot fields precede it.
//!
//! # Why the schema WHERE rules are enforced here
//!
//! IfcPropertySet states ExistsName and UniquePropertyNames.
//! A nameless pset cannot be looked up, and duplicate property
//! names make a lookup ambiguous: both parse, both corrupt.
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use ifc_model::guid::Guid;
use ifc_schema::ifc4;

use crate::{PropertyError, PropertyResult};

/// `IfcPropertySet` slots.
pub mod pset_slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`). Required by the ExistsName rule.
    pub const NAME: usize = 2;
    /// `Description` (from `IfcRoot`).
    pub const DESCRIPTION: usize = 3;
    /// `HasProperties`. Required.
    pub const HAS_PROPERTIES: usize = 4;
}

/// `IfcPropertySingleValue` slots.
pub mod single_value_slot {
    /// `Name` (from `IfcProperty`). Required.
    pub const NAME: usize = 0;
    /// `Specification` (from `IfcProperty`).
    pub const SPECIFICATION: usize = 1;
    /// `NominalValue`.
    pub const NOMINAL_VALUE: usize = 2;
    /// `Unit`.
    pub const UNIT: usize = 3;
}

/// `IfcRelDefinesByProperties` slots.
pub mod defines_slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `RelatedObjects`. Required.
    pub const RELATED_OBJECTS: usize = 4;
    /// `RelatingPropertyDefinition`. Required.
    pub const RELATING_DEFINITION: usize = 5;
}

/// Stage an `IfcPropertySingleValue`.
///
/// `value` is an `IfcValue`: a measure-wrapped scalar such as
/// `Value::Typed { type_name: IFCLENGTHMEASURE, .. }`. A bare
/// literal is legal but dimensionally meaningless, so the caller
/// chooses; this crate will not invent a measure.
///
/// `specification` is `IfcProperty.Specification`: prose describing
/// what the property means, kept distinct from its value.
///
/// # Errors
///
/// Refuses a blank name: IfcProperty.Name is required, and a
/// whitespace-only name satisfies EXISTS while naming nothing.
pub fn add_property_single_value(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    value: Option<Value>,
    unit: Option<EntityId>,
) -> PropertyResult<EntityId> {
    if name.trim().is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSINGLEVALUE",
            attribute: "Name",
            value: name.to_owned(),
        });
    }
    let mut attributes = vec![Value::Null; single_value_slot::UNIT + 1];
    attributes[single_value_slot::NAME] = Value::Text(name.into());
    attributes[single_value_slot::SPECIFICATION] = optional_text(specification);
    attributes[single_value_slot::NOMINAL_VALUE] = value.unwrap_or(Value::Null);
    attributes[single_value_slot::UNIT] = unit.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCPROPERTYSINGLEVALUE", attributes)))
}

/// Stage an `IfcPropertySet`.
///
/// # Errors
///
/// Refuses a blank name (ExistsName), an empty property list
/// (HasProperties is required), and duplicate property names
/// (UniquePropertyNames).
///
/// Properties are passed as `(name, id)` pairs rather than bare ids:
/// the uniqueness rule is stated over names, and a staged entity
/// cannot be read back out of the transaction to recover them.
pub fn add_property_set(
    tx: &mut Transaction,
    global_id: &str,
    name: &str,
    description: Option<&str>,
    properties: &[(&str, EntityId)],
) -> PropertyResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSET",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    if name.trim().is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSET",
            attribute: "Name",
            value: name.to_owned(),
        });
    }
    if properties.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSET",
            attribute: "HasProperties",
            value: String::from("an empty set"),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for (property_name, _) in properties {
        if !seen.insert(*property_name) {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYSET",
                attribute: "HasProperties",
                value: (*property_name).to_owned(),
            });
        }
    }
    let refs = properties.iter().map(|(_, id)| Value::Ref(*id)).collect();
    let mut attributes = vec![Value::Null; pset_slot::HAS_PROPERTIES + 1];
    attributes[pset_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[pset_slot::NAME] = Value::Text(name.into());
    attributes[pset_slot::DESCRIPTION] = optional_text(description);
    attributes[pset_slot::HAS_PROPERTIES] = Value::List(refs);
    Ok(tx.create(Entity::new("IFCPROPERTYSET", attributes)))
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.into()))
}

/// Stage an `IfcRelDefinesByProperties` attaching a set to objects.
///
/// # Errors
///
/// Refuses an empty object list, and any object whose type is an
/// `IfcTypeObject` subtype. The schema states NoRelatedTypeObject:
/// a type carries properties through IfcRelDefinesByType instead, and
/// attaching here would be read by nothing that walks type properties.
///
/// Needs the model because the rule is stated over the related
/// objects types, which only the committed model knows.
pub fn attach_property_set(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    property_set: EntityId,
) -> PropertyResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYPROPERTIES",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    if objects.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYPROPERTIES",
            attribute: "RelatedObjects",
            value: String::from("an empty set"),
        });
    }
    let schema = ifc4();
    for object in objects {
        let Some(entity) = model.get(*object) else {
            return Err(PropertyError::MissingEntity { id: *object });
        };
        if schema.is_a(entity.type_name.as_ref(), "IFCTYPEOBJECT") {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCRELDEFINESBYPROPERTIES",
                attribute: "RelatedObjects",
                value: entity.type_name.to_string(),
            });
        }
    }
    let refs = objects.iter().copied().map(Value::Ref).collect();
    let mut attributes = vec![Value::Null; defines_slot::RELATING_DEFINITION + 1];
    attributes[defines_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[defines_slot::RELATED_OBJECTS] = Value::List(refs);
    attributes[defines_slot::RELATING_DEFINITION] = Value::Ref(property_set);
    Ok(tx.create(Entity::new("IFCRELDEFINESBYPROPERTIES", attributes)))
}

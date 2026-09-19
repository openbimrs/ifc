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

/// `IfcPropertyEnumeratedValue` slots.
pub mod enumerated_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `EnumerationValues`.
    pub const VALUES: usize = 2;
    /// `EnumerationReference`.
    pub const REFERENCE: usize = 3;
}

/// `IfcPropertyBoundedValue` slots.
pub mod bounded_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `UpperBoundValue`.
    pub const UPPER: usize = 2;
    /// `LowerBoundValue`.
    pub const LOWER: usize = 3;
    /// `Unit`.
    pub const UNIT: usize = 4;
    /// `SetPointValue`.
    pub const SET_POINT: usize = 5;
}

/// `IfcPropertyListValue` slots.
pub mod list_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `ListValues`.
    pub const VALUES: usize = 2;
    /// `Unit`.
    pub const UNIT: usize = 3;
}

/// `IfcPropertyTableValue` slots.
pub mod table_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `DefiningValues`, the independent variable.
    pub const DEFINING: usize = 2;
    /// `DefinedValues`, the dependent variable.
    pub const DEFINED: usize = 3;
    /// `Expression`.
    pub const EXPRESSION: usize = 4;
    /// `DefiningUnit`.
    pub const DEFINING_UNIT: usize = 5;
    /// `DefinedUnit`.
    pub const DEFINED_UNIT: usize = 6;
    /// `CurveInterpolation`.
    pub const INTERPOLATION: usize = 7;
}

/// `IfcPropertyReferenceValue` slots.
pub mod reference_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `UsageName`.
    pub const USAGE_NAME: usize = 2;
    /// `PropertyReference`.
    pub const REFERENCE: usize = 3;
}

/// `IfcComplexProperty` slots.
pub mod complex_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Specification`.
    pub const SPECIFICATION: usize = 1;
    /// `UsageName`. Required.
    pub const USAGE_NAME: usize = 2;
    /// `HasProperties`. Required.
    pub const HAS_PROPERTIES: usize = 3;
}

/// Refuse a blank property name.
///
/// `IfcProperty.Name` is required and is the key every lookup uses. A
/// whitespace-only name satisfies the schema's EXISTS check and still
/// names nothing.
fn require_name(entity: &'static str, name: &str) -> PropertyResult<()> {
    if name.trim().is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity,
            attribute: "Name",
            value: name.to_owned(),
        });
    }
    Ok(())
}

/// Stage an `IfcPropertyEnumeratedValue`.
///
/// # Errors
///
/// Refuses a blank name, and an empty value list: the schema types
/// `EnumerationValues` as `LIST [1:?]`, so an empty aggregate is
/// malformed where omission is legal.
pub fn add_property_enumerated_value(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    values: Option<Vec<Value>>,
    enumeration: Option<EntityId>,
) -> PropertyResult<EntityId> {
    require_name("IFCPROPERTYENUMERATEDVALUE", name)?;
    if let Some(values) = values.as_ref() {
        if values.is_empty() {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYENUMERATEDVALUE",
                attribute: "EnumerationValues",
                value: "empty".to_owned(),
            });
        }
    }
    let mut attributes = vec![Value::Null; enumerated_slot::REFERENCE + 1];
    attributes[enumerated_slot::NAME] = Value::Text(name.into());
    attributes[enumerated_slot::SPECIFICATION] = optional_text(specification);
    attributes[enumerated_slot::VALUES] = values.map_or(Value::Null, Value::List);
    attributes[enumerated_slot::REFERENCE] = enumeration.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCPROPERTYENUMERATEDVALUE", attributes)))
}

/// The declared measure of a value, when it wears one.
///
/// `Value::Typed { type_name, .. }` is how a measure-wrapped scalar
/// reaches this crate. A bare literal has no measure and compares
/// equal to any other bare literal.
fn measure_of(value: &Value) -> Option<&str> {
    match value {
        Value::Typed { type_name, .. } => Some(type_name.as_ref()),
        _ => None,
    }
}

/// Stage an `IfcPropertyBoundedValue`.
///
/// # Errors
///
/// Refuses a blank name, and bounds whose measures disagree. The
/// schema states SameUnitUpperLower, SameUnitLowerSet and
/// SameUnitUpperSet: mixing a length lower bound with a mass upper
/// bound parses and then compares two different quantities.
pub fn add_property_bounded_value(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    upper: Option<Value>,
    lower: Option<Value>,
    set_point: Option<Value>,
    unit: Option<EntityId>,
) -> PropertyResult<EntityId> {
    require_name("IFCPROPERTYBOUNDEDVALUE", name)?;
    let stated: Vec<(&str, &Value)> = [
        ("UpperBoundValue", upper.as_ref()),
        ("LowerBoundValue", lower.as_ref()),
        ("SetPointValue", set_point.as_ref()),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| (label, value)))
    .collect();
    if let Some(((_, first), rest)) = stated.split_first() {
        let want = measure_of(first);
        for (label, value) in rest {
            if measure_of(value) != want {
                return Err(PropertyError::AuthoringInvalid {
                    entity: "IFCPROPERTYBOUNDEDVALUE",
                    attribute: "SameUnit",
                    value: (*label).to_owned(),
                });
            }
        }
    }
    let mut attributes = vec![Value::Null; bounded_slot::SET_POINT + 1];
    attributes[bounded_slot::NAME] = Value::Text(name.into());
    attributes[bounded_slot::SPECIFICATION] = optional_text(specification);
    attributes[bounded_slot::UPPER] = upper.unwrap_or(Value::Null);
    attributes[bounded_slot::LOWER] = lower.unwrap_or(Value::Null);
    attributes[bounded_slot::UNIT] = unit.map_or(Value::Null, Value::Ref);
    attributes[bounded_slot::SET_POINT] = set_point.unwrap_or(Value::Null);
    Ok(tx.create(Entity::new("IFCPROPERTYBOUNDEDVALUE", attributes)))
}

/// Stage an `IfcPropertyListValue`.
///
/// # Errors
///
/// Refuses a blank name and an empty list, which the schema types as
/// `LIST [1:?]`.
pub fn add_property_list_value(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    values: Option<Vec<Value>>,
    unit: Option<EntityId>,
) -> PropertyResult<EntityId> {
    require_name("IFCPROPERTYLISTVALUE", name)?;
    if let Some(values) = values.as_ref() {
        if values.is_empty() {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYLISTVALUE",
                attribute: "ListValues",
                value: "empty".to_owned(),
            });
        }
    }
    let mut attributes = vec![Value::Null; list_slot::UNIT + 1];
    attributes[list_slot::NAME] = Value::Text(name.into());
    attributes[list_slot::SPECIFICATION] = optional_text(specification);
    attributes[list_slot::VALUES] = values.map_or(Value::Null, Value::List);
    attributes[list_slot::UNIT] = unit.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCPROPERTYLISTVALUE", attributes)))
}

/// Authored fields for `IfcPropertyTableValue`.
///
/// A struct rather than a parameter list: the entity carries two
/// columns, two units and two free-text fields, and positional
/// arguments of the same types are easy to transpose silently.
#[derive(Debug, Default)]
pub struct TableValueDraft<'a> {
    /// `Name`. Required, and the key every lookup uses.
    pub name: &'a str,
    /// `Specification`.
    pub specification: Option<&'a str>,
    /// `DefiningValues`, the independent variable.
    pub defining: Option<Vec<Value>>,
    /// `DefinedValues`, the dependent variable.
    pub defined: Option<Vec<Value>>,
    /// `DefiningUnit`.
    pub defining_unit: Option<EntityId>,
    /// `DefinedUnit`.
    pub defined_unit: Option<EntityId>,
    /// `Expression`, the closed form when one exists.
    pub expression: Option<&'a str>,
    /// `CurveInterpolation`, an `IfcCurveInterpolationEnum` token.
    pub interpolation: Option<&'a str>,
}

/// Stage an `IfcPropertyTableValue`.
///
/// A lookup table: `defining` is the independent variable, `defined`
/// the dependent one, and row `i` pairs `defining[i]` with
/// `defined[i]`.
///
/// # Errors
///
/// Refuses a blank name. Enforces the schema's three table rules:
/// WR21, the two columns must be the same length, since a row whose
/// input has no output is not a row; WR22 and WR23, each column must
/// be homogeneous, since a column mixing lengths and masses cannot be
/// interpolated.
pub fn add_property_table_value(
    tx: &mut Transaction,
    draft: TableValueDraft<'_>,
) -> PropertyResult<EntityId> {
    let TableValueDraft {
        name,
        specification,
        defining,
        defined,
        defining_unit,
        defined_unit,
        expression,
        interpolation,
    } = draft;
    require_name("IFCPROPERTYTABLEVALUE", name)?;
    let sizes = (
        defining.as_ref().map(Vec::len),
        defined.as_ref().map(Vec::len),
    );
    if let (Some(left), Some(right)) = sizes {
        if left != right {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYTABLEVALUE",
                attribute: "DefiningValues",
                value: format!("{left} defining against {right} defined"),
            });
        }
    }
    for (label, column) in [
        ("DefiningValues", defining.as_ref()),
        ("DefinedValues", defined.as_ref()),
    ] {
        let Some(column) = column else { continue };
        if column.is_empty() {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYTABLEVALUE",
                attribute: label,
                value: "empty".to_owned(),
            });
        }
        let want = measure_of(&column[0]);
        if column.iter().any(|value| measure_of(value) != want) {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCPROPERTYTABLEVALUE",
                attribute: label,
                value: "mixed measures".to_owned(),
            });
        }
    }
    let mut attributes = vec![Value::Null; table_slot::INTERPOLATION + 1];
    attributes[table_slot::NAME] = Value::Text(name.into());
    attributes[table_slot::SPECIFICATION] = optional_text(specification);
    attributes[table_slot::DEFINING] = defining.map_or(Value::Null, Value::List);
    attributes[table_slot::DEFINED] = defined.map_or(Value::Null, Value::List);
    attributes[table_slot::EXPRESSION] = optional_text(expression);
    attributes[table_slot::DEFINING_UNIT] = defining_unit.map_or(Value::Null, Value::Ref);
    attributes[table_slot::DEFINED_UNIT] = defined_unit.map_or(Value::Null, Value::Ref);
    attributes[table_slot::INTERPOLATION] =
        interpolation.map_or(Value::Null, |token| Value::Enum(token.into()));
    Ok(tx.create(Entity::new("IFCPROPERTYTABLEVALUE", attributes)))
}

/// Stage an `IfcPropertyReferenceValue`.
///
/// Points at another entity rather than carrying a scalar: a table,
/// a document reference, a time series.
///
/// # Errors
///
/// Refuses a blank name.
pub fn add_property_reference_value(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    usage_name: Option<&str>,
    reference: Option<EntityId>,
) -> PropertyResult<EntityId> {
    require_name("IFCPROPERTYREFERENCEVALUE", name)?;
    let mut attributes = vec![Value::Null; reference_slot::REFERENCE + 1];
    attributes[reference_slot::NAME] = Value::Text(name.into());
    attributes[reference_slot::SPECIFICATION] = optional_text(specification);
    attributes[reference_slot::USAGE_NAME] = optional_text(usage_name);
    attributes[reference_slot::REFERENCE] = reference.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCPROPERTYREFERENCEVALUE", attributes)))
}

/// Stage an `IfcComplexProperty`.
///
/// Groups properties under one name, so a U-value can carry its own
/// measurement conditions as nested properties.
///
/// # Errors
///
/// Refuses a blank name or usage name, an empty `HasProperties` (the
/// schema types it `SET [1:?]`), duplicate nested names, and a
/// property listed as its own child. The schema states WR21 as
/// `SELF :=: temp`, forbidding direct self-containment: a complex
/// property inside itself is a cycle that a recursive reader cannot
/// terminate on.
pub fn add_complex_property(
    tx: &mut Transaction,
    name: &str,
    specification: Option<&str>,
    usage_name: &str,
    properties: &[(&str, EntityId)],
) -> PropertyResult<EntityId> {
    require_name("IFCCOMPLEXPROPERTY", name)?;
    if usage_name.trim().is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCCOMPLEXPROPERTY",
            attribute: "UsageName",
            value: usage_name.to_owned(),
        });
    }
    if properties.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCCOMPLEXPROPERTY",
            attribute: "HasProperties",
            value: "empty".to_owned(),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for (property_name, _) in properties {
        if !seen.insert(*property_name) {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCCOMPLEXPROPERTY",
                attribute: "HasProperties",
                value: (*property_name).to_owned(),
            });
        }
    }
    let refs = properties.iter().map(|(_, id)| Value::Ref(*id)).collect();
    let mut attributes = vec![Value::Null; complex_slot::HAS_PROPERTIES + 1];
    attributes[complex_slot::NAME] = Value::Text(name.into());
    attributes[complex_slot::SPECIFICATION] = optional_text(specification);
    attributes[complex_slot::USAGE_NAME] = Value::Text(usage_name.into());
    attributes[complex_slot::HAS_PROPERTIES] = Value::List(refs);
    // WR21 forbids SELF among HasProperties. The id is only known after
    // staging, so the check runs here; the refusal abandons the staged
    // record, which commit never sees because the caller drops the error.
    let own = tx.create(Entity::new("IFCCOMPLEXPROPERTY", attributes));
    if properties.iter().any(|(_, id)| *id == own) {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCCOMPLEXPROPERTY",
            attribute: "HasProperties",
            value: "self reference".to_owned(),
        });
    }
    Ok(own)
}

/// `IfcElementQuantity` slots.
pub mod element_quantity_slot {
    /// `GlobalId` (from `IfcRoot`). Required.
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Description` (from `IfcRoot`).
    pub const DESCRIPTION: usize = 3;
    /// `MethodOfMeasurement`.
    pub const METHOD: usize = 4;
    /// `Quantities`. Required.
    pub const QUANTITIES: usize = 5;
}

/// `IfcPhysicalComplexQuantity` slots.
pub mod complex_quantity_slot {
    /// `Name`. Required.
    pub const NAME: usize = 0;
    /// `Description`.
    pub const DESCRIPTION: usize = 1;
    /// `HasQuantities`. Required.
    pub const HAS_QUANTITIES: usize = 2;
    /// `Discrimination`. Required.
    pub const DISCRIMINATION: usize = 3;
    /// `Quality`.
    pub const QUALITY: usize = 4;
    /// `Usage`.
    pub const USAGE: usize = 5;
}

/// Stage an `IfcElementQuantity`.
///
/// The quantity-takeoff counterpart of a property set: a named group
/// of measured quantities attached to an object through
/// [`attach_property_set`], which accepts any
/// `IfcPropertySetDefinition`.
///
/// # Errors
///
/// Refuses a malformed GUID and an empty quantity list, which the
/// schema types as `SET [1:?]`.
pub fn add_element_quantity(
    tx: &mut Transaction,
    global_id: &str,
    name: &str,
    method_of_measurement: Option<&str>,
    quantities: &[EntityId],
) -> PropertyResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCELEMENTQUANTITY",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    require_name("IFCELEMENTQUANTITY", name)?;
    if quantities.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCELEMENTQUANTITY",
            attribute: "Quantities",
            value: "empty".to_owned(),
        });
    }
    let mut attributes = vec![Value::Null; element_quantity_slot::QUANTITIES + 1];
    attributes[element_quantity_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[element_quantity_slot::NAME] = Value::Text(name.into());
    attributes[element_quantity_slot::METHOD] = optional_text(method_of_measurement);
    attributes[element_quantity_slot::QUANTITIES] =
        Value::List(quantities.iter().copied().map(Value::Ref).collect());
    Ok(tx.create(Entity::new("IFCELEMENTQUANTITY", attributes)))
}

/// Stage an `IfcPhysicalComplexQuantity`.
///
/// Groups quantities that share a discrimination, so a wall's gross
/// and net areas can sit under one heading.
///
/// # Errors
///
/// Refuses a blank name or discrimination, and an empty
/// `HasQuantities`, typed `SET [1:?]` by the schema.
pub fn add_physical_complex_quantity(
    tx: &mut Transaction,
    name: &str,
    description: Option<&str>,
    quantities: &[EntityId],
    discrimination: &str,
) -> PropertyResult<EntityId> {
    require_name("IFCPHYSICALCOMPLEXQUANTITY", name)?;
    if discrimination.trim().is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPHYSICALCOMPLEXQUANTITY",
            attribute: "Discrimination",
            value: discrimination.to_owned(),
        });
    }
    if quantities.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPHYSICALCOMPLEXQUANTITY",
            attribute: "HasQuantities",
            value: "empty".to_owned(),
        });
    }
    let mut attributes = vec![Value::Null; complex_quantity_slot::USAGE + 1];
    attributes[complex_quantity_slot::NAME] = Value::Text(name.into());
    attributes[complex_quantity_slot::DESCRIPTION] = optional_text(description);
    attributes[complex_quantity_slot::HAS_QUANTITIES] =
        Value::List(quantities.iter().copied().map(Value::Ref).collect());
    attributes[complex_quantity_slot::DISCRIMINATION] = Value::Text(discrimination.into());
    Ok(tx.create(Entity::new("IFCPHYSICALCOMPLEXQUANTITY", attributes)))
}

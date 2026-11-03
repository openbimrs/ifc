//! Where a value states its own unit: the slots of the property, quantity
//! and measure-with-unit entities that carry a unit attribute, and the
//! entities that state it for another (an enumeration, a time series, a
//! table column).
//!
//! Shared by the length-unit conversion (#469), which keeps such values,
//! and SI input (#638), which converts into that unit.

use ifc_schema::Schema;

/// Value attributes and the unit attribute that states their unit; `*` for
/// every attribute but the unit.
pub(crate) const UNIT_SLOTS: &[(&str, &[&str], &str)] = &[
    ("IFCPROPERTYSINGLEVALUE", &["NominalValue"], "Unit"),
    (
        "IFCPROPERTYBOUNDEDVALUE",
        &["UpperBoundValue", "LowerBoundValue", "SetPointValue"],
        "Unit",
    ),
    ("IFCPROPERTYLISTVALUE", &["ListValues"], "Unit"),
    ("IFCPROPERTYENUMERATION", &["EnumerationValues"], "Unit"),
    ("IFCPROPERTYTABLEVALUE", &["DefiningValues"], "DefiningUnit"),
    ("IFCPROPERTYTABLEVALUE", &["DefinedValues"], "DefinedUnit"),
    ("IFCPHYSICALSIMPLEQUANTITY", &["*"], "Unit"),
    ("IFCMEASUREWITHUNIT", &["ValueComponent"], "UnitComponent"),
];

/// Attributes typed as a temperature that state a temperature *difference*
/// (`IfcStructuralLoadTemperature`, `DeltaT*` in IFC2X3 spelt `DeltaT_*`;
/// `IFC2X3_TC1.exp`, `IFC4.exp`, `IFC4X3_ADD2.exp`). A difference converts by
/// the unit's factor alone: 10 K is 10 degrees Celsius, not -263.15.
#[cfg(feature = "authoring")]
pub(crate) const TEMPERATURE_DIFFERENCES: &[(&str, &str)] = &[
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaTConstant"),
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaTY"),
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaTZ"),
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaT_Constant"),
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaT_Y"),
    ("IFCSTRUCTURALLOADTEMPERATURE", "DeltaT_Z"),
];

/// Whether `attribute_name` of `entity_type` states a temperature
/// difference.
#[cfg(feature = "authoring")]
pub(crate) fn is_temperature_difference(
    schema: &Schema,
    entity_type: &str,
    attribute_name: &str,
) -> bool {
    TEMPERATURE_DIFFERENCES.iter().any(|(owner, name)| {
        schema.is_a(entity_type, owner) && name.eq_ignore_ascii_case(attribute_name)
    })
}

/// Where an attribute's unit is stated, per entity type and attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rule {
    /// Nowhere: the project unit.
    Project,
    /// By this attribute of the same entity.
    UnitAttribute(&'static str),
    /// By the `IfcPropertyEnumeration` the entity references.
    Enumeration,
    /// By the time series listing the entity.
    Series,
    /// Per cell, by the table column.
    Rows,
}

/// The rule for `attribute_name` of `entity_type`.
pub(crate) fn rule(schema: &Schema, entity_type: &str, attribute_name: &str) -> Rule {
    for (owner, values, unit) in UNIT_SLOTS {
        if !schema.is_a(entity_type, owner) || attribute_name.eq_ignore_ascii_case(unit) {
            continue;
        }
        if values.contains(&"*")
            || values
                .iter()
                .any(|value| value.eq_ignore_ascii_case(attribute_name))
        {
            return Rule::UnitAttribute(unit);
        }
    }
    let named = |name: &str| attribute_name.eq_ignore_ascii_case(name);
    if schema.is_a(entity_type, "IFCPROPERTYENUMERATEDVALUE") && named("EnumerationValues") {
        Rule::Enumeration
    } else if (schema.is_a(entity_type, "IFCTIMESERIESVALUE")
        || schema.is_a(entity_type, "IFCIRREGULARTIMESERIESVALUE"))
        && named("ListValues")
    {
        Rule::Series
    } else if schema.is_a(entity_type, "IFCTABLEROW") && named("RowCells") {
        Rule::Rows
    } else {
        Rule::Project
    }
}

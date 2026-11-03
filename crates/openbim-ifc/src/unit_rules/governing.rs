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

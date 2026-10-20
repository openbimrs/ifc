//! Which values state their own unit, and so keep it.
//!
//! A value is in the project unit unless the schema gives it another:
//!
//! - directly, by a unit attribute of the same entity: `IfcPropertySingleValue`,
//!   `IfcPropertyBoundedValue`, `IfcPropertyListValue` and
//!   `IfcPropertyEnumeration` (`Unit`), `IfcPropertyTableValue`
//!   (`DefiningUnit` for `DefiningValues`, `DefinedUnit` for
//!   `DefinedValues`), every `IfcPhysicalSimpleQuantity` (`Unit`) and
//!   `IfcMeasureWithUnit` (`UnitComponent`);
//! - through another entity: an `IfcPropertyEnumeratedValue` through its
//!   `EnumerationReference`'s `Unit`, an `IfcTimeSeriesValue` (or IFC2X3/IFC4
//!   `IfcIrregularTimeSeriesValue`) through the `Unit` of the time series
//!   listing it, and an `IfcTableRow`'s cells through their `IfcTableColumn`'s
//!   `Unit` (IFC4 on; IFC2X3 tables have no columns).
//!
//! A value two owners state differently (a time series value or table row
//! listed by two owners, one with a unit and one without) is refused.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

use super::report::{LengthUnitRefusal, RefusalReason};
use super::space::{attribute, reference};

/// Value attributes and the unit attribute that states their unit; `*` for
/// every attribute but the unit.
const UNIT_SLOTS: &[(&str, &[&str], &str)] = &[
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

/// Whether a slot's values are in the project unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Governed {
    /// In the project unit.
    Project,
    /// In a unit of their own.
    Explicit,
    /// Per element of the slot's list: `true` for a unit of their own.
    PerElement(Vec<bool>),
}

/// The values whose unit another entity states, found in one pass.
pub(super) struct Scope {
    /// Time series values: whether their series states a unit.
    series: HashMap<EntityId, bool>,
    /// Table rows: per cell, whether its column states a unit.
    rows: HashMap<EntityId, Vec<bool>>,
}

impl Scope {
    pub(super) fn build(
        schema: &Schema,
        model: &Model,
        refusals: &mut Vec<LengthUnitRefusal>,
    ) -> Self {
        let mut scope = Self {
            series: HashMap::new(),
            rows: HashMap::new(),
        };
        for (id, entity) in model.iter() {
            if schema.is_a(&entity.type_name, "IFCTIMESERIES") {
                let explicit = has_value(attribute(schema, model, id, "Unit"));
                for value in refs(attribute(schema, model, id, "Values")) {
                    if let Some(previous) = scope.series.insert(value, explicit) {
                        if previous != explicit {
                            refusals.push(ambiguous(model, value));
                        }
                    }
                }
            } else if schema.is_a(&entity.type_name, "IFCTABLE") {
                let columns: Vec<bool> = refs(attribute(schema, model, id, "Columns"))
                    .into_iter()
                    .map(|column| has_value(attribute(schema, model, column, "Unit")))
                    .collect();
                for row in refs(attribute(schema, model, id, "Rows")) {
                    if let Some(previous) = scope.rows.insert(row, columns.clone()) {
                        if previous != columns {
                            refusals.push(ambiguous(model, row));
                        }
                    }
                }
            }
        }
        scope
    }

    /// Whether the values a slot governed by `rule` holds on `entity` are
    /// in the project unit.
    pub(super) fn governed(
        &self,
        rule: Rule,
        schema: &Schema,
        model: &Model,
        entity: EntityId,
    ) -> Governed {
        match rule {
            Rule::Project => Governed::Project,
            Rule::UnitAttribute(unit) => {
                explicit_if(has_value(attribute(schema, model, entity, unit)))
            }
            Rule::Enumeration => explicit_if(
                reference(schema, model, entity, "EnumerationReference").is_some_and(
                    |enumeration| has_value(attribute(schema, model, enumeration, "Unit")),
                ),
            ),
            Rule::Series => self
                .series
                .get(&entity)
                .map_or(Governed::Project, |explicit| explicit_if(*explicit)),
            Rule::Rows => self.rows.get(&entity).map_or(Governed::Project, |columns| {
                Governed::PerElement(columns.clone())
            }),
        }
    }
}

/// Where an attribute's unit is stated, per entity type and attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rule {
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
pub(super) fn rule(schema: &Schema, entity_type: &str, attribute_name: &str) -> Rule {
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

fn explicit_if(explicit: bool) -> Governed {
    if explicit {
        Governed::Explicit
    } else {
        Governed::Project
    }
}

fn has_value(value: Option<&Value>) -> bool {
    !matches!(value, None | Some(Value::Null | Value::Derived))
}

fn refs(value: Option<&Value>) -> Vec<EntityId> {
    match value {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn ambiguous(model: &Model, entity: EntityId) -> LengthUnitRefusal {
    LengthUnitRefusal {
        entity,
        entity_type: model
            .get(entity)
            .map_or_else(|| Arc::from("(missing)"), |e| e.type_name.clone()),
        attribute: None,
        reason: RefusalReason::AmbiguousUnit,
    }
}

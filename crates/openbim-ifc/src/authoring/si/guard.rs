//! An SI batch must not change the units it converts into (#638).
//!
//! Numbers convert into the units the model had before the batch. An
//! operation that changes the project's unit assignment, a unit or the
//! `IfcMeasureWithUnit` behind a conversion-based unit, or that re-points a
//! value holder's `Unit`, would make those conversions wrong by the time the
//! batch commits, so it is refused. Creating records is allowed: a new unit
//! changes nothing until something names it, and an SI value naming it is
//! refused as created in the batch.

use ifc_model::{EntityId, Value};
use ifc_schema::Schema;

use super::SiInput;
use crate::authoring::overlay::Overlay;
use crate::authoring::si_refusal::SiInputRefusal;
use crate::authoring::{AuthorOp, AuthoringFailure as F, NamedValues};
use crate::unit_rules::governing::UNIT_SLOTS;
use crate::unit_rules::space::attribute;

/// Entities that are, or compose, a unit.
const UNIT_ENTITIES: [&str; 6] = [
    "IFCNAMEDUNIT",
    "IFCDERIVEDUNIT",
    "IFCDERIVEDUNITELEMENT",
    "IFCMONETARYUNIT",
    "IFCDIMENSIONALEXPONENTS",
    "IFCUNITASSIGNMENT",
];

/// Unit attributes of value holders beside [`UNIT_SLOTS`].
const OTHER_UNIT_SLOTS: [(&str, &str); 3] = [
    ("IFCTIMESERIES", "Unit"),
    ("IFCTABLECOLUMN", "Unit"),
    // Re-pointing the enumeration re-points the unit its values are in.
    ("IFCPROPERTYENUMERATEDVALUE", "EnumerationReference"),
];

fn unit_changed(entity_type: &str, attribute: Option<&str>, entity: Option<EntityId>) -> F {
    F::SiInput {
        entity_type: Some(entity_type.to_owned()),
        attribute: attribute.map(str::to_owned),
        reason: SiInputRefusal::UnitsChanged { entity },
    }
}

impl SiInput<'_> {
    /// Refuse `op` when it changes the units an SI batch converts into.
    pub(super) fn guard(&self, view: &Overlay<'_>, op: &AuthorOp) -> Result<(), F> {
        let schema = self.classifier.schema();
        match op {
            AuthorOp::Create {
                type_name,
                attributes,
            } => self.writes_units(schema, type_name, attributes, None, true),
            AuthorOp::Project { attributes, .. } => {
                self.writes_units(schema, "IFCPROJECT", attributes, None, true)
            }
            AuthorOp::Edit { entity, attributes } => {
                let Some(record) = view.get(*entity) else {
                    // The planner refuses a missing entity.
                    return Ok(());
                };
                let current = record.type_name.clone();
                self.changes_record(schema, view, *entity, &current, false)?;
                self.writes_units(schema, &current, attributes, Some(*entity), false)
            }
            AuthorOp::ReassignClass {
                entity,
                type_name,
                attributes,
                ..
            } => {
                if let Some(record) = view.get(*entity) {
                    let current = record.type_name.clone();
                    self.changes_record(schema, view, *entity, &current, true)?;
                }
                if UNIT_ENTITIES
                    .iter()
                    .any(|unit| schema.is_a(type_name, unit))
                {
                    return Err(unit_changed(type_name, None, Some(*entity)));
                }
                self.writes_units(schema, type_name, attributes, Some(*entity), false)
            }
            AuthorOp::Remove { entity } => match view.get(*entity) {
                Some(record) => {
                    let current = record.type_name.clone();
                    self.changes_record(schema, view, *entity, &current, true)
                }
                None => Ok(()),
            },
            // A purge removes what nothing uses, and a unit nothing names yet
            // is among it: the guard cannot tell, so an SI batch has none.
            AuthorOp::Purge(_) => Err(F::SiInput {
                entity_type: None,
                attribute: None,
                reason: SiInputRefusal::UnitsChanged { entity: None },
            }),
            _ => Ok(()),
        }
    }

    /// Refuse editing, retyping or removing a unit, or the measure a
    /// conversion-based unit is defined by.
    fn changes_record(
        &self,
        schema: &Schema,
        view: &Overlay<'_>,
        entity: EntityId,
        entity_type: &str,
        replaces: bool,
    ) -> Result<(), F> {
        // Retyping or removing a context that states the units takes them
        // away; editing its other attributes does not.
        let states_units = replaces
            && (schema.is_a(entity_type, "IFCCONTEXT") || schema.is_a(entity_type, "IFCPROJECT"))
            && attribute(schema, view, entity, "UnitsInContext")
                .is_some_and(|value| !matches!(value, Value::Null | Value::Derived));
        let is_unit = UNIT_ENTITIES
            .iter()
            .any(|unit| schema.is_a(entity_type, unit));
        let defines_unit = schema.is_a(entity_type, "IFCMEASUREWITHUNIT")
            && view.referrers(entity).into_iter().any(|referrer| {
                view.get(referrer)
                    .is_some_and(|record| schema.is_a(&record.type_name, "IFCCONVERSIONBASEDUNIT"))
            });
        if is_unit || defines_unit || states_units {
            Err(unit_changed(entity_type, None, Some(entity)))
        } else {
            Ok(())
        }
    }

    /// Refuse writing the project's unit assignment, and, outside a create,
    /// a value holder's `Unit`.
    fn writes_units(
        &self,
        schema: &Schema,
        entity_type: &str,
        attributes: &NamedValues,
        entity: Option<EntityId>,
        creates: bool,
    ) -> Result<(), F> {
        for (name, _) in attributes {
            let context =
                schema.is_a(entity_type, "IFCCONTEXT") || schema.is_a(entity_type, "IFCPROJECT");
            if context && name.eq_ignore_ascii_case("UnitsInContext") {
                return Err(unit_changed(entity_type, Some("UnitsInContext"), entity));
            }
            if creates {
                continue;
            }
            let holder = UNIT_SLOTS
                .iter()
                .map(|(owner, _, unit)| (*owner, *unit))
                .chain(OTHER_UNIT_SLOTS)
                .any(|(owner, unit)| {
                    schema.is_a(entity_type, owner) && name.eq_ignore_ascii_case(unit)
                });
            if holder {
                return Err(unit_changed(entity_type, Some(name), entity));
            }
        }
        Ok(())
    }
}

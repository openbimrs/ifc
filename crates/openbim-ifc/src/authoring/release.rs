//! Moving an appended entity into the target model's release, and checking
//! that the two models measure alike (#479).
//!
//! An attribute moves by name, never by position: IFC4 inserted
//! attributes into supertypes (`IfcWall.PredefinedType`), so a slot of one
//! release is not the same slot of another. What the target cannot hold is
//! refused with the reason, for the caller to report as
//! [`IncompatibleRelease`](super::AuthoringFailure::IncompatibleRelease).

use ifc_model::{Entity, Value};
use ifc_schema::{Schema, TypeKind};

use super::append::Source;
use super::kinds::Kinds;
use super::overlay::Overlay;
use super::plan::Planner;
use super::AuthoringFailure as F;
use crate::attribute_slots;

/// Whether both releases declare `type_name` with the same attributes in
/// the same order.
pub(super) fn same_layout(source: &Schema, target: &Schema, type_name: &str) -> bool {
    let names = |schema: &Schema| -> Option<Vec<String>> {
        schema.entity(type_name)?;
        Some(
            schema
                .attributes(type_name)
                .iter()
                .map(|attribute| attribute.name.to_ascii_lowercase())
                .collect(),
        )
    };
    match (names(source), names(target)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// `entity`, read in `source`'s release, as `target` lays it out; the
/// reason when the target cannot hold it.
pub(super) fn migrate(source: &Schema, target: &Schema, entity: &Entity) -> Result<Entity, String> {
    let t = &entity.type_name;
    let declared = target
        .entity(t)
        .ok_or_else(|| format!("{} does not declare {t}", target.name()))?;
    if declared.abstract_ {
        return Err(format!("{t} is abstract in {}", target.name()));
    }
    let from = attribute_slots(source, t)
        .map_err(|_| format!("{} does not declare {t}", source.name()))?;
    let into = attribute_slots(target, t).map_err(|error| error.to_string())?;
    let mut attributes = vec![Value::Null; into.len()];
    let mut filled = vec![false; into.len()];
    for slot in &from {
        let value = entity.attributes.get(slot.index).unwrap_or(&Value::Null);
        let unset = matches!(value, Value::Null | Value::Derived);
        let Some(found) = into
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(slot.name))
        else {
            if unset {
                continue;
            }
            return Err(format!(
                "{} has no attribute {t}.{}, which is set",
                target.name(),
                slot.name
            ));
        };
        filled[found.index] = true;
        if found.derived {
            if !unset {
                return Err(format!(
                    "{t}.{} is derived in {}, and set in the source",
                    slot.name,
                    target.name()
                ));
            }
            attributes[found.index] = Value::Derived;
            continue;
        }
        if matches!(value, Value::Derived) {
            return Err(format!(
                "{t}.{} is derived in {}, and has a value of its own in {}",
                slot.name,
                source.name(),
                target.name()
            ));
        }
        admitted(target, found.type_name, value)
            .map_err(|detail| format!("{t}.{}: {detail}", slot.name))?;
        attributes[found.index] = value.clone();
    }
    for slot in &into {
        if slot.derived && !filled[slot.index] {
            attributes[slot.index] = Value::Derived;
        }
    }
    Ok(Entity::new(entity.type_name.clone(), attributes))
}

/// Whether the target release knows `value`'s enumeration constants and
/// typed values, against the attribute's declared type.
fn admitted(schema: &Schema, declared: &str, value: &Value) -> Result<(), String> {
    match value {
        Value::List(items) => items
            .iter()
            .try_for_each(|item| admitted(schema, declared, item)),
        Value::Typed { type_name, value } => {
            if schema.type_def(type_name).is_none() && schema.entity(type_name).is_none() {
                return Err(format!("{} has no type {type_name}", schema.name()));
            }
            admitted(schema, type_name, value)
        }
        Value::Enum(constant) => match members(schema, declared) {
            Some(members)
                if !members
                    .iter()
                    .any(|member| member.eq_ignore_ascii_case(constant.trim_matches('.'))) =>
            {
                Err(format!(
                    "{} has no {declared} constant .{}.",
                    schema.name(),
                    constant.trim_matches('.')
                ))
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

/// The constants of an enumeration type, through defined-type aliases.
fn members<'s>(schema: &'s Schema, declared: &str) -> Option<&'s [String]> {
    let mut current = declared.to_owned();
    for _ in 0..16 {
        match &schema.type_def(&current)?.kind {
            TypeKind::Enumeration(members) => return Some(members),
            TypeKind::Defined(alias) => current = alias.trim().to_owned(),
            _ => return None,
        }
    }
    None
}

/// The unit types the models must agree on, with the SI unit a model
/// that declares none is read in.
const MEASURED: &[(&str, &str)] = &[("LENGTHUNIT", "METRE"), ("PLANEANGLEUNIT", "RADIAN")];

impl Planner<'_, '_> {
    /// Refuse an append between models whose length or plane angle units
    /// differ.
    pub(super) fn check_units(&self, source: &Source<'_>) -> Result<(), F> {
        let target_kinds = Kinds::new(self.schema);
        for (unit_type, si) in MEASURED {
            let from = project_unit(&source.view, &source.kinds, unit_type);
            let into = project_unit(&self.view, &target_kinds, unit_type);
            let same = match (from, into) {
                (Some(s), Some(t)) => {
                    let mut memo = std::collections::BTreeMap::new();
                    self.equal(source, &std::collections::BTreeMap::new(), &mut memo, s, t)
                }
                (None, None) => true,
                (Some(s), None) => describe(&source.view, &source.kinds, s) == *si,
                (None, Some(t)) => describe(&self.view, &target_kinds, t) == *si,
            };
            if !same {
                let name = |unit: Option<_>, view: &Overlay<'_>, kinds: &Kinds<'_>| {
                    unit.map_or_else(|| format!("none ({si})"), |id| describe(view, kinds, id))
                };
                return Err(F::UnitMismatch {
                    unit_type: (*unit_type).to_owned(),
                    source: name(from, &source.view, &source.kinds),
                    target: name(into, &self.view, &target_kinds),
                });
            }
        }
        Ok(())
    }
}

/// The unit of `unit_type` the model's `IfcProject` assigns.
fn project_unit(
    view: &Overlay<'_>,
    kinds: &Kinds<'_>,
    unit_type: &str,
) -> Option<ifc_model::EntityId> {
    let project = *view.ids_of_type("IFCPROJECT").first()?;
    let assignment = kinds.reference(view.get(project)?, "UnitsInContext")?;
    kinds
        .references(view.get(assignment)?, "Units")
        .into_iter()
        .find(|unit| {
            view.get(*unit).is_some_and(|entity| {
                kinds
                    .slot(&entity.type_name, "UnitType")
                    .and_then(|slot| entity.attributes.get(slot))
                    .is_some_and(|value| {
                        matches!(value, Value::Enum(e) if e.trim_matches('.').eq_ignore_ascii_case(unit_type))
                    })
            })
        })
}

/// A unit as a reader names it: `MILLI METRE`, `METRE`, `DEGREE`.
fn describe(view: &Overlay<'_>, kinds: &Kinds<'_>, unit: ifc_model::EntityId) -> String {
    let Some(entity) = view.get(unit) else {
        return unit.to_string();
    };
    let text = |name: &str| {
        kinds
            .slot(&entity.type_name, name)
            .and_then(|slot| entity.attributes.get(slot))
            .and_then(|value| match value.unwrap_typed() {
                Value::Enum(e) => Some(e.trim_matches('.').to_owned()),
                Value::Text(t) => Some(t.to_string()),
                _ => None,
            })
    };
    match (text("Prefix"), text("Name")) {
        (Some(prefix), Some(name)) => format!("{prefix} {name}"),
        (None, Some(name)) => name,
        _ => entity.type_name.to_string(),
    }
}

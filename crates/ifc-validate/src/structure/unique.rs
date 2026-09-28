//! EXPRESS `UNIQUE` rules (#111).
//!
//! A `UNIQUE` clause states that the named attributes are unique, jointly,
//! across every instance of the declaring entity and its subtypes: IFC4's
//! `IfcApplication.UR2` forbids two applications sharing both
//! `ApplicationFullName` and `Version`. The schema tables record each clause
//! with its label, so every one the declared release states is checked,
//! against that release's own clauses (IFC2X3 declares 17, IFC4 and IFC4X3
//! four).
//!
//! `IfcRoot.UR1` (unique `GlobalId`) is the one clause checked elsewhere: it
//! is registered as the `global.UniqueGlobalId` rule, whose id callers
//! already filter on, and checking it here too would report every duplicate
//! twice.
//!
//! An instance whose attribute in the clause is unset (`$`) or derived (`*`)
//! takes no part: its value is indeterminate, so it cannot be shown to
//! collide with another.

use std::collections::HashMap;

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};

/// The clause checked by the `global.UniqueGlobalId` where-rule instead.
const CHECKED_AS_GLOBAL_RULE: (&str, &str) = ("IfcRoot", "UR1");

/// Reports every instance that repeats another's values for a `UNIQUE`
/// clause of the declared release.
pub fn unique_rules(model: &Model, schema: &Schema, report: &mut Report) {
    for declaring in schema.entities() {
        for rule in &declaring.unique_rules {
            let label = rule.label.as_deref().unwrap_or("unlabelled");
            if declaring
                .name
                .eq_ignore_ascii_case(CHECKED_AS_GLOBAL_RULE.0)
                && label == CHECKED_AS_GLOBAL_RULE.1
            {
                continue;
            }
            // `SELF\X.Y` names an inherited attribute: the slot is `Y`'s.
            let names: Vec<&str> = rule
                .attributes
                .iter()
                .map(|name| name.rsplit(['.', '\\']).next().unwrap_or(name).trim())
                .collect();
            check(model, schema, &declaring.name, label, &names, report);
        }
    }
}

fn check(
    model: &Model,
    schema: &Schema,
    declaring: &str,
    label: &str,
    names: &[&str],
    report: &mut Report,
) {
    let mut instances: Vec<EntityId> = Vec::new();
    for (type_name, _) in model.type_histogram() {
        if schema.is_a(type_name, declaring) {
            instances.extend_from_slice(model.ids_of_type(type_name));
        }
    }
    instances.sort_unstable();
    let mut seen: HashMap<String, EntityId> = HashMap::new();
    for id in instances {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let layout = schema.attribute_names(&entity.type_name);
        let slots: Option<Vec<usize>> = names
            .iter()
            .map(|name| {
                layout
                    .iter()
                    .position(|slot| slot.eq_ignore_ascii_case(name))
            })
            .collect();
        let Some(slots) = slots else {
            continue;
        };
        let values: Option<Vec<&Value>> = slots
            .iter()
            .map(|&slot| {
                entity
                    .attributes
                    .get(slot)
                    .filter(|value| !matches!(value, Value::Null | Value::Derived))
            })
            .collect();
        let Some(values) = values else {
            continue;
        };
        let key = format!("{values:?}");
        match seen.get(&key) {
            Some(first) => report.push(Finding::error(
                "structure.unique.violation",
                Path::Attribute {
                    entity: id,
                    index: slots[0],
                    name: Some(names[0].to_owned()),
                },
                format!(
                    "{declaring}.{label}: {} must be unique across {declaring}, \
                     and {first} already holds the same value",
                    names.join(", ")
                ),
            )),
            None => {
                seen.insert(key, id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Entity;
    use std::sync::Arc;

    fn application(schema: &Schema, identifier: &str, name: &str, version: &str) -> Entity {
        let names = schema.attribute_names("IfcApplication");
        let mut values = vec![Value::Null; names.len()];
        let mut set = |attribute: &str, value: Value| {
            let slot = names.iter().position(|n| *n == attribute).unwrap();
            values[slot] = value;
        };
        set("ApplicationIdentifier", Value::Text(Arc::from(identifier)));
        set("ApplicationFullName", Value::Text(Arc::from(name)));
        set("Version", Value::Text(Arc::from(version)));
        Entity::new("IFCAPPLICATION", values)
    }

    fn run(entities: Vec<Entity>) -> Vec<String> {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        for entity in entities {
            model.push(entity);
        }
        let mut report = Report::new();
        unique_rules(&model, schema, &mut report);
        report
            .findings()
            .iter()
            .map(|f| f.message.clone())
            .collect()
    }

    #[test]
    fn a_repeated_clause_is_reported_once_per_repeat() {
        let schema = ifc_schema::ifc4();
        assert!(run(vec![
            application(schema, "a", "App", "1"),
            application(schema, "b", "App", "2"),
        ])
        .is_empty());
        let found = run(vec![
            application(schema, "a", "App", "1"),
            application(schema, "a", "Other", "1"),
        ]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("IfcApplication.UR1"), "{found:?}");
        let found = run(vec![
            application(schema, "a", "App", "1"),
            application(schema, "b", "App", "1"),
        ]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("IfcApplication.UR2"), "{found:?}");
    }
}

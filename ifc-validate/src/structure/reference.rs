//! References that point at nothing, or at the wrong kind of thing.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};

/// Reports every reference whose target the model does not contain.
///
/// A dangling reference is unambiguously a defect: Part 21 `#42` names an
/// instance in the same exchange structure, so a missing target means the file
/// was truncated, badly merged, or written by a tool that dropped an entity it
/// still pointed at.
///
/// Traversal is by ascending entity id and, within an entity, by ascending
/// slot, so findings arrive in a stable order.
pub fn dangling_references(model: &Model, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        for (index, value) in entity.attributes.iter().enumerate() {
            let mut missing = Vec::new();
            value.for_each_ref(&mut |target| {
                if model.get(target).is_none() {
                    missing.push(target);
                }
            });
            missing.sort_unstable();
            missing.dedup();
            for target in missing {
                report.push(Finding::error(
                    "structure.reference.dangling",
                    Path::Attribute {
                        entity: id,
                        index,
                        name: None,
                    },
                    format!("references {target}, which the file does not contain"),
                ));
            }
        }
    }
}

/// Reports references whose target is not of the declared entity type.
///
/// Only checked where the schema declares an entity-typed slot: the parser
/// records the declared type token, and if that token names an entity, the
/// target must be that entity or a subtype of it. Slots declared as SELECTs or
/// defined types are left to [`crate::type_check`], which understands them.
pub fn wrong_kind_references(model: &Model, schema: &Schema, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let declared = schema.attributes(&entity.type_name);
        for (index, value) in entity.attributes.iter().enumerate() {
            let Some(attribute) = declared.get(index) else {
                continue;
            };
            // Only entity-typed slots are checkable here. A SELECT names a
            // type declaration, not an entity, and is handled elsewhere.
            if schema.entity(&attribute.type_name).is_none() {
                continue;
            }
            let Value::Ref(target) = value.unwrap_typed() else {
                continue;
            };
            let Some(target_entity) = model.get(*target) else {
                continue; // already reported as dangling
            };
            if !schema.is_a(&target_entity.type_name, &attribute.type_name) {
                report.push(Finding::error(
                    "structure.reference.wrong_type",
                    Path::Attribute {
                        entity: id,
                        index,
                        name: Some(attribute.name.clone()),
                    },
                    format!(
                        "declared {} but {target} is {}",
                        attribute.type_name, target_entity.type_name
                    ),
                ));
            }
        }
    }
}

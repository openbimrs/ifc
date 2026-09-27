//! References that point at nothing, or at the wrong kind of thing.

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};
use crate::type_check::element_type;

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
/// Checked wherever the schema expects an entity: an entity-typed slot, the
/// members of an aggregate of entities (`SET OF IfcProduct`), and aggregates
/// reached through a type that aliases one. The target must be that entity or
/// a subtype of it. References in SELECT slots are left to
/// [`crate::type_check`], which walks the SELECT's closure; a value that is
/// not a reference at all is `type.entity.expected_reference`. A target whose
/// type the tables do not declare is not judged: it is
/// `type.entity.unknown`'s finding.
///
/// One finding per distinct offending target in a slot, in ascending order.
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
            let mut offenders: Vec<(EntityId, String, &str)> = Vec::new();
            for (expected, target) in expected_references(schema, &attribute.type_name, value) {
                // Only entity expectations are judged here; a SELECT names a
                // type declaration, not an entity.
                if schema.entity(&expected).is_none() {
                    continue;
                }
                let Some(target_entity) = model.get(target) else {
                    continue; // already reported as dangling
                };
                // A target of a type these tables do not declare -- usually a
                // later release's entity -- gives no basis for a subtype
                // verdict; `type.entity.unknown` already reports it.
                if schema.entity(&target_entity.type_name).is_none() {
                    continue;
                }
                if !schema.is_a(&target_entity.type_name, &expected) {
                    offenders.push((target, expected, &target_entity.type_name));
                }
            }
            offenders.sort_unstable();
            offenders.dedup();
            for (target, expected, actual) in offenders {
                report.push(Finding::error(
                    "structure.reference.wrong_type",
                    Path::Attribute {
                        entity: id,
                        index,
                        name: Some(attribute.name.clone()),
                    },
                    format!("declared {expected} but {target} is {actual}"),
                ));
            }
        }
    }
}

/// How deeply nested aggregates are searched for references.
///
/// Matches the value check's bound: IFC nests aggregates three deep at
/// most, and the depth of a written value is under the file's control.
const MAX_NESTING: usize = 8;

/// Every reference in `value`, paired with the type its position expects.
///
/// A reference directly in the slot expects the slot's `declared` type; one
/// inside an aggregate expects the aggregate's element type, and one inside
/// a typed wrapper expects the wrapper's type. The expected type may be an
/// entity, a SELECT, or a type that takes no references at all -- telling
/// those apart is the caller's business.
pub(crate) fn expected_references(
    schema: &Schema,
    declared: &str,
    value: &Value,
) -> Vec<(String, EntityId)> {
    let mut found = Vec::new();
    collect(schema, declared, value, 0, &mut found);
    found
}

fn collect(
    schema: &Schema,
    declared: &str,
    value: &Value,
    depth: usize,
    found: &mut Vec<(String, EntityId)>,
) {
    match value {
        Value::Ref(target) => found.push((declared.to_string(), *target)),
        Value::List(items) if depth < MAX_NESTING => {
            let element = element_type(schema, declared);
            for item in items {
                collect(schema, &element, item, depth + 1, found);
            }
        }
        Value::Typed { type_name, value } => collect(schema, type_name, value, depth, found),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A target of a type the tables do not declare is not accused of being
    /// the wrong kind -- an IFC4X3 `IfcReferent` read against IFC4 tables --
    /// while a declared wrong-kind target in the same aggregate still is.
    #[test]
    fn an_undeclared_target_type_is_not_judged() {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            ifc_model::Entity::new("IFCREFERENT", Vec::new()),
        );
        model.insert(
            EntityId(2),
            ifc_model::Entity::new("IFCPROPERTYSET", Vec::new()),
        );
        let mut attributes =
            vec![Value::Null; schema.attributes("IFCRELDEFINESBYPROPERTIES").len()];
        attributes[4] = Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))]);
        model.insert(
            EntityId(3),
            ifc_model::Entity::new("IFCRELDEFINESBYPROPERTIES", attributes),
        );
        let mut report = Report::new();
        wrong_kind_references(&model, schema, &mut report);
        let messages: Vec<&str> = report
            .findings()
            .iter()
            .map(|finding| finding.message.as_str())
            .collect();
        assert_eq!(
            messages,
            ["declared IfcObjectDefinition but #2 is IFCPROPERTYSET"]
        );
    }

    /// References in aggregates expect the element type, including one
    /// reached through a type that aliases an aggregate.
    #[test]
    fn references_are_paired_with_the_type_their_position_expects() {
        let schema = ifc_schema::ifc4();
        let set = Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))]);
        assert_eq!(
            expected_references(schema, "IfcProduct", &set),
            [
                ("IfcProduct".to_string(), EntityId(1)),
                ("IfcProduct".to_string(), EntityId(2))
            ]
        );
        let typed = Value::Typed {
            type_name: "IFCPROPERTYSETDEFINITIONSET".into(),
            value: Box::new(Value::List(vec![Value::Ref(EntityId(3))])),
        };
        assert_eq!(
            expected_references(schema, "IfcPropertySetDefinitionSelect", &typed),
            [("IfcPropertySetDefinition".to_string(), EntityId(3))]
        );
    }
}

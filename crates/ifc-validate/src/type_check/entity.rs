//! Entity-level type checking: unknown types, abstract instantiation, values.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use super::defined::{check_all, Mismatch};
use super::select::admits_entity;
use crate::report::{Finding, Path, Report};
use crate::structure::expected_references;

/// Reports entities whose type the schema does not declare.
///
/// A warning, not an error: the model is deliberately schema-agnostic and an
/// unknown type round-trips intact. It is still worth saying, because it is
/// usually a schema-version mismatch -- an IFC4X3 file read against IFC4.
pub fn unknown_entity_types(model: &Model, schema: &Schema, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        if schema.entity(&entity.type_name).is_none() {
            report.push(Finding::warning(
                "type.entity.unknown",
                Path::Entity(id),
                format!(
                    "{} is not declared by schema {}",
                    entity.type_name,
                    schema.name()
                ),
            ));
        }
    }
}

/// Reports instances of `ABSTRACT` entities.
///
/// EXPRESS `ABSTRACT SUPERTYPE` means the entity may not be instantiated
/// directly; only its concrete subtypes may appear in a file.
pub fn abstract_instances(model: &Model, schema: &Schema, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let Some(definition) = schema.entity(&entity.type_name) else {
            continue;
        };
        if definition.abstract_ {
            report.push(Finding::error(
                "type.entity.abstract",
                Path::Entity(id),
                format!("{} is abstract and cannot be instantiated", definition.name),
            ));
        }
    }
}

/// Reports values that do not match their slot's declared type.
///
/// Aggregate members are checked against the element type, and every
/// reference in a SELECT position against the SELECT's entity alternatives.
pub fn attribute_types(model: &Model, schema: &Schema, report: &mut Report) {
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
            let path = || Path::Attribute {
                entity: id,
                index,
                name: Some(attribute.name.clone()),
            };
            select_references(model, schema, &attribute.type_name, value, &path, report);
            for mismatch in check_all(schema, &attribute.type_name, value) {
                report.push(finding(
                    mismatch,
                    path(),
                    &attribute.type_name,
                    &attribute.name,
                ));
            }
        }
    }
}

/// The finding one mismatch in the slot at `path` is reported as.
fn finding(mismatch: Mismatch, path: Path, declared: &str, attribute: &str) -> Finding {
    match mismatch {
        Mismatch::Primitive { expected, actual } => Finding::error(
            "type.scalar.mismatch",
            path,
            format!("{declared} is {expected}, the file wrote {actual}"),
        ),
        Mismatch::FixedWidth { expected, actual } => Finding::error(
            "type.scalar.fixed_width",
            path,
            format!("{declared} is STRING({expected}) FIXED, the file wrote {actual} characters"),
        ),
        Mismatch::EnumMember {
            member,
            declared: members,
        } => Finding::error(
            "type.enumeration.member",
            path,
            format!(
                "{member} is not a member of {declared} ({})",
                if members.is_empty() {
                    "no members declared".to_string()
                } else {
                    members.join(", ")
                }
            ),
        ),
        Mismatch::SelectMember { written, select } => Finding::error(
            "type.select.member",
            path,
            format!("{written} is not a member of {select}"),
        ),
        Mismatch::ExpectedReference { declared, actual } => Finding::error(
            "type.entity.expected_reference",
            path,
            format!("{attribute} takes a reference to {declared}, the file wrote {actual}"),
        ),
        Mismatch::TypedOutsideSelect { written, declared } => Finding::error(
            "type.typed.outside_select",
            path,
            format!(
                "{declared} is not a SELECT, so its value is written bare, \
                 not as the typed parameter {written}(...)"
            ),
        ),
        Mismatch::TypedWrongType { written, declared } => Finding::error(
            "type.typed.wrong_type",
            path,
            format!("{written} is not {declared}, and {declared} is not a SELECT"),
        ),
        Mismatch::UntypedSelectValue { select, actual } => Finding::error(
            "type.select.untyped",
            path,
            format!(
                "{select} is a SELECT, so a value that is not a reference is \
                 written as a typed parameter; the file wrote {actual}"
            ),
        ),
    }
}

/// Reports references in SELECT positions whose target the SELECT does not
/// admit.
///
/// A reference is admitted when its target is, or inherits from, an entity
/// anywhere in the SELECT's closure. One finding per distinct offending
/// target, in ascending order. A dangling target is `structure`'s finding,
/// and a target of a type the schema does not declare is
/// `type.entity.unknown`'s; neither is judged again here.
fn select_references(
    model: &Model,
    schema: &Schema,
    declared: &str,
    value: &Value,
    path: &dyn Fn() -> Path,
    report: &mut Report,
) {
    let mut offenders = Vec::new();
    for (expected, target) in expected_references(schema, declared, value) {
        let Some(target_entity) = model.get(target) else {
            continue;
        };
        if schema.entity(&target_entity.type_name).is_none() {
            continue;
        }
        if admits_entity(schema, &expected, &target_entity.type_name) == Some(false) {
            offenders.push((target, expected, &*target_entity.type_name));
        }
    }
    offenders.sort_unstable();
    offenders.dedup();
    for (target, select, actual) in offenders {
        report.push(Finding::error(
            "type.select.member",
            path(),
            format!("{target} is {actual}, which is not a member of {select}"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Entity;

    /// One slot with two independent violations yields two findings on the
    /// same path (#215).
    #[test]
    fn a_slot_reports_each_independent_violation() {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        let names = schema.attribute_names("IFCCARTESIANPOINTLIST3D");
        let mut attributes = vec![Value::Null; names.len()];
        attributes[0] = Value::List(vec![
            Value::List(vec![
                Value::Text("x".into()),
                Value::Real(0.0),
                Value::Real(0.0),
            ]),
            Value::List(vec![
                Value::Real(0.0),
                Value::Typed {
                    type_name: "IFCLENGTHMEASURE".into(),
                    value: Box::new(Value::Real(1.0)),
                },
                Value::Real(0.0),
            ]),
        ]);
        let id = model.push(Entity::new("IFCCARTESIANPOINTLIST3D", attributes));
        let mut report = Report::new();
        attribute_types(&model, schema, &mut report);
        let rules: Vec<&str> = report
            .findings()
            .iter()
            .filter(|finding| matches!(finding.path, Path::Attribute { entity, index: 0, .. } if entity == id))
            .map(|finding| finding.rule.as_str())
            .collect();
        assert_eq!(rules, ["type.scalar.mismatch", "type.typed.outside_select"]);
    }
}

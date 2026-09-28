//! Entity-level type checking: unknown types, abstract instantiation, values.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use super::defined::{check, Mismatch};
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
            let Some(mismatch) = check(schema, &attribute.type_name, value) else {
                continue;
            };
            let path = path();
            let finding = match mismatch {
                Mismatch::Primitive { expected, actual } => Finding::error(
                    "type.scalar.mismatch",
                    path,
                    format!(
                        "{} is {expected}, the file wrote {actual}",
                        attribute.type_name
                    ),
                ),
                Mismatch::FixedWidth { expected, actual } => Finding::error(
                    "type.scalar.fixed_width",
                    path,
                    format!(
                        "{} is STRING({expected}) FIXED, the file wrote {actual} characters",
                        attribute.type_name
                    ),
                ),
                Mismatch::EnumMember { member, declared } => Finding::error(
                    "type.enumeration.member",
                    path,
                    format!(
                        "{member} is not a member of {} ({})",
                        attribute.type_name,
                        if declared.is_empty() {
                            "no members declared".to_string()
                        } else {
                            declared.join(", ")
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
                    format!(
                        "{} takes a reference to {declared}, the file wrote {actual}",
                        attribute.name
                    ),
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
            };
            report.push(finding);
        }
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

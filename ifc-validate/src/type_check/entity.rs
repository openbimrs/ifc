//! Entity-level type checking: unknown types, abstract instantiation, values.

use ifc_model::Model;
use ifc_schema::Schema;

use super::defined::{check, Mismatch};
use crate::report::{Finding, Path, Report};

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
            let Some(mismatch) = check(schema, &attribute.type_name, value) else {
                continue;
            };
            let path = Path::Attribute {
                entity: id,
                index,
                name: Some(attribute.name.clone()),
            };
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
            };
            report.push(finding);
        }
    }
}

//! Required attributes that are absent, and derived slots written as values.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};

/// Reports required attributes stated as `$`, and slot-count mismatches.
///
/// # Three distinct defects, three distinct rules
///
/// - A non-`OPTIONAL` attribute written `$` is missing data.
/// - A record with fewer slots than the schema declares is truncated.
/// - A record with more slots carries data the schema cannot explain.
///
/// # Derived slots
///
/// EXPRESS `DERIVE` redeclares an inherited attribute as computed. Part 21
/// writes such a slot `*`, never `$` and never a value. Both mistakes are
/// reported, because `*` and `$` mean different things: `*` says "the schema
/// computes this", `$` says "nobody stated it".
pub fn required_attributes(model: &Model, schema: &Schema, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let Some(definition) = schema.entity(&entity.type_name) else {
            // An entity the schema does not declare. Reported by type_check;
            // there is nothing to compare slots against here.
            continue;
        };
        let declared = schema.attributes(&entity.type_name);
        if entity.attributes.len() != declared.len() {
            report.push(Finding::error(
                "structure.required.slot_count",
                Path::Entity(id),
                format!(
                    "{} declares {} attributes, the record has {}",
                    definition.name,
                    declared.len(),
                    entity.attributes.len()
                ),
            ));
            continue;
        }
        for (index, (value, attribute)) in entity.attributes.iter().zip(declared.iter()).enumerate()
        {
            let is_derived = is_derived_slot(schema, &entity.type_name, &attribute.name);
            let path = || Path::Attribute {
                entity: id,
                index,
                name: Some(attribute.name.clone()),
            };
            match value {
                Value::Derived if !is_derived => report.push(Finding::error(
                    "structure.required.not_derived",
                    path(),
                    "written as `*` but the schema does not derive it",
                )),
                Value::Null if is_derived => report.push(Finding::error(
                    "structure.required.derived_as_null",
                    path(),
                    "a derived slot must be written `*`, not `$`",
                )),
                Value::Null if !attribute.optional => report.push(Finding::error(
                    "structure.required.missing",
                    path(),
                    "required attribute is not stated",
                )),
                _ if is_derived && !matches!(value, Value::Derived) => report.push(Finding::error(
                    "structure.required.derived_has_value",
                    path(),
                    "a derived slot must be written `*`, not a value",
                )),
                _ => {}
            }
        }
    }
}

/// Whether any entity in the supertype chain declares `name` derived.
///
/// A subtype redeclares an inherited attribute as derived, so the declaration
/// can sit anywhere above the attribute's owner.
fn is_derived_slot(schema: &Schema, type_name: &str, attribute: &str) -> bool {
    if schema
        .entity(type_name)
        .is_some_and(|definition| definition.is_derived(attribute))
    {
        return true;
    }
    schema
        .supertypes(type_name)
        .iter()
        .filter_map(|name| schema.entity(name))
        .any(|definition| definition.is_derived(attribute))
}

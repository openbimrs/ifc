//! Rules implemented natively, without an expression evaluator.
//!
//! # Why these three
//!
//! Each is checkable from structure alone -- entity counts, id uniqueness,
//! declared types -- so implementing them needs no EXPRESS evaluator, and
//! each catches a defect that appears in real files:
//!
//! - `IfcSingleProjectInstance`: two `IfcProject` entities means two
//!   coordinate systems and two unit assignments, and nothing says which is
//!   authoritative.
//! - `UniqueGlobalId` (`IfcRoot.UR1`): duplicate GUIDs break every external
//!   reference into the file, because a GUID stops identifying one thing.
//! - `NoRelatedTypeObject`: attaching a property set to a *type* through the
//!   occurrence relation puts the same properties on every occurrence of that
//!   type, silently and unintentionally.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};

/// `IfcSingleProjectInstance`: `SIZEOF(IfcProject) <= 1`.
///
/// A global rule, so its path is the file rather than any single entity --
/// no one `IfcProject` is at fault.
pub fn single_project_instance(model: &Model, report: &mut Report) {
    let projects: Vec<_> = model.of_type("IFCPROJECT").map(|(id, _)| id).collect();
    if projects.len() > 1 {
        let ids: Vec<String> = projects.iter().map(ToString::to_string).collect();
        report.push(Finding::error(
            "global.IfcSingleProjectInstance",
            Path::File,
            format!(
                "a file declares exactly one IfcProject; found {}: {}",
                projects.len(),
                ids.join(", ")
            ),
        ));
    }
}

/// `IfcRoot.UR1`: `GlobalId` is unique across the file.
///
/// Reported against the *later* entity: the first occurrence is not the
/// error, the repeat is. Deterministic because entity iteration is ordered.
pub fn unique_global_id(model: &Model, schema: &Schema, report: &mut Report) {
    let mut seen: std::collections::HashMap<&str, ifc_model::EntityId> =
        std::collections::HashMap::new();
    let mut entries: Vec<(ifc_model::EntityId, &str)> = Vec::new();
    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IfcRoot") {
            continue;
        }
        let Some(Value::Text(guid)) = entity.attribute(0) else {
            continue;
        };
        entries.push((id, guid.as_ref()));
    }
    entries.sort_by_key(|(id, _)| id.0);
    for (id, guid) in entries {
        if let Some(first) = seen.get(guid) {
            report.push(Finding::error(
                "global.UniqueGlobalId",
                Path::Attribute {
                    entity: id,
                    index: 0,
                    name: Some("GlobalId".into()),
                },
                format!("GlobalId {guid} is already used by {first}"),
            ));
        } else {
            seen.insert(guid, id);
        }
    }
}

/// `IfcRelDefinesByProperties.NoRelatedTypeObject`.
///
/// The relation's `RelatedObjects` (slot 4) must contain no `IfcTypeObject`.
/// Type-level property sets travel through `IfcRelDefinesByType` instead.
pub fn no_related_type_object(model: &Model, schema: &Schema, report: &mut Report) {
    for (id, entity) in model.of_type("IFCRELDEFINESBYPROPERTIES") {
        let Some(related) = entity.attribute(4) else {
            continue;
        };
        let mut offenders = Vec::new();
        related.for_each_ref(&mut |target| {
            if let Some(object) = model.get(target) {
                if schema.is_a(&object.type_name, "IfcTypeObject") {
                    offenders.push(target);
                }
            }
        });
        offenders.sort_by_key(|target| target.0);
        for offender in offenders {
            report.push(Finding::error(
                "IfcRelDefinesByProperties.NoRelatedTypeObject",
                Path::Attribute {
                    entity: id,
                    index: 4,
                    name: Some("RelatedObjects".into()),
                },
                format!(
                    "{offender} is an IfcTypeObject; type property sets \
                     attach through IfcRelDefinesByType"
                ),
            ));
        }
    }
}

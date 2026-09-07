//! Rules implemented natively, without an expression evaluator.
//!
//! # Why these checks
//!
//! Each is checkable from direct model structure or scalar values -- entity
//! counts, identity fields, references, integer bounds, and declared types --
//! so implementing them needs no general EXPRESS evaluator. They catch defects
//! that appear in real files while keeping the unsupported boundary explicit:
//!
//! - `IfcSingleProjectInstance`: two `IfcProject` entities means two
//!   coordinate systems and two unit assignments, and nothing says which is
//!   authoritative.
//! - `UniqueGlobalId` (`IfcRoot.UR1`): duplicate GUIDs break every external
//!   reference into the file, because a GUID stops identifying one thing.
//! - `NoRelatedTypeObject`: attaching a property set to a *type* through the
//!   occurrence relation puts the same properties on every occurrence of that
//!   type, silently and unintentionally.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, SchemaVersion};

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

fn attribute_index(schema: &Schema, type_name: &str, attribute: &str) -> Option<usize> {
    schema
        .attribute_names(type_name)
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
}

fn attribute_path(entity: EntityId, index: usize, name: &str) -> Path {
    Path::Attribute {
        entity,
        index,
        name: Some(name.into()),
    }
}

fn is_present(value: Option<&Value>) -> bool {
    !matches!(value, None | Some(Value::Null | Value::Derived))
}

/// `IfcExternalReference.WR1`: at least one external identity field exists.
pub fn external_reference_identity(model: &Model, schema: &Schema, report: &mut Report) {
    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IfcExternalReference") {
            continue;
        }
        let identified = ["Identification", "ItemReference", "Location", "Name"]
            .iter()
            .filter_map(|name| attribute_index(schema, &entity.type_name, name))
            .any(|index| is_present(entity.attribute(index)));
        if !identified {
            report.push(Finding::error(
                "IfcExternalReference.WR1",
                Path::Entity(id),
                "external reference has no identification, location, or name",
            ));
        }
    }
}

/// Sequence endpoints must refer to different processes.
pub fn sequence_endpoints_differ(model: &Model, schema: &Schema, report: &mut Report) {
    let rule = if schema.version() == Some(SchemaVersion::Ifc2x3) {
        "IfcRelSequence.WR1"
    } else {
        "IfcRelSequence.AvoidInconsistentSequence"
    };
    for (id, entity) in model.of_type("IFCRELSEQUENCE") {
        let Some(relating_index) = attribute_index(schema, &entity.type_name, "RelatingProcess")
        else {
            continue;
        };
        let Some(related_index) = attribute_index(schema, &entity.type_name, "RelatedProcess")
        else {
            continue;
        };
        let endpoints = (
            entity.attribute(relating_index).and_then(Value::as_ref_id),
            entity.attribute(related_index).and_then(Value::as_ref_id),
        );
        if let (Some(relating), Some(related)) = endpoints {
            if relating == related {
                report.push(Finding::error(
                    rule,
                    attribute_path(id, related_index, "RelatedProcess"),
                    format!("sequence endpoints both refer to {related}"),
                ));
            }
        }
    }
}

fn no_self_reference(
    model: &Model,
    schema: &Schema,
    relation: &str,
    rule: &str,
    report: &mut Report,
) {
    for (id, entity) in model.of_type(relation) {
        let Some(parent_index) = attribute_index(schema, &entity.type_name, "RelatingObject")
        else {
            continue;
        };
        let Some(children_index) = attribute_index(schema, &entity.type_name, "RelatedObjects")
        else {
            continue;
        };
        let Some(parent) = entity.attribute(parent_index).and_then(Value::as_ref_id) else {
            continue;
        };
        let mut includes_parent = false;
        if let Some(children) = entity.attribute(children_index) {
            children.for_each_ref(&mut |child| includes_parent |= child == parent);
        }
        if includes_parent {
            report.push(Finding::error(
                rule,
                attribute_path(id, children_index, "RelatedObjects"),
                format!("related objects contain their own parent {parent}"),
            ));
        }
    }
}

/// IFC4/IFC4X3 decomposition and nesting relations cannot contain their parent.
pub fn decomposition_has_no_self_reference(model: &Model, schema: &Schema, report: &mut Report) {
    if !matches!(
        schema.version(),
        Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)
    ) {
        return;
    }
    no_self_reference(
        model,
        schema,
        "IFCRELAGGREGATES",
        "IfcRelAggregates.NoSelfReference",
        report,
    );
    no_self_reference(
        model,
        schema,
        "IFCRELNESTS",
        "IfcRelNests.NoSelfReference",
        report,
    );
}

/// IFC4/IFC4X3 material-layer priority, when set, is in the inclusive 0..=100 range.
pub fn normalized_material_priority(model: &Model, schema: &Schema, report: &mut Report) {
    if !matches!(
        schema.version(),
        Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)
    ) {
        return;
    }
    for (id, entity) in model.of_type("IFCMATERIALLAYER") {
        let Some(index) = attribute_index(schema, &entity.type_name, "Priority") else {
            continue;
        };
        let Some(value) = entity
            .attribute(index)
            .filter(|value| is_present(Some(value)))
            .and_then(|value| value.unwrap_typed().as_i64())
        else {
            continue;
        };
        if !(0..=100).contains(&value) {
            report.push(Finding::error(
                "IfcMaterialLayer.NormalizedPriority",
                attribute_path(id, index, "Priority"),
                format!("priority {value} is outside the inclusive 0..=100 range"),
            ));
        }
    }
}

/// The four `IfcRelAssigns` subtypes and the attribute naming their
/// single relating end.
///
/// `IfcRelAssignsToGroupByFactor` is listed separately from
/// `IfcRelAssignsToGroup`: `of_type` matches exact type names, so the
/// subtype is invisible to a query for its parent and would otherwise
/// go unchecked.
const ASSIGNMENT_RELATING: [(&str, &str); 4] = [
    ("IFCRELASSIGNSTOACTOR", "RelatingActor"),
    ("IFCRELASSIGNSTOPROCESS", "RelatingProcess"),
    ("IFCRELASSIGNSTOPRODUCT", "RelatingProduct"),
    ("IFCRELASSIGNSTOGROUPBYFACTOR", "RelatingGroup"),
];

/// IFC4/IFC4X3: an assignment relationship cannot assign an object to
/// itself.
///
/// The schema states this per subtype as `NoSelfReference`, each naming
/// its own relating attribute, so the check is driven by the table above
/// rather than assuming a shared slot.
pub fn assignment_has_no_self_reference(model: &Model, schema: &Schema, report: &mut Report) {
    if !matches!(
        schema.version(),
        Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)
    ) {
        return;
    }
    for (relation, relating_attribute) in ASSIGNMENT_RELATING {
        let rule = match relation {
            "IFCRELASSIGNSTOACTOR" => "IfcRelAssignsToActor.NoSelfReference",
            "IFCRELASSIGNSTOPROCESS" => "IfcRelAssignsToProcess.NoSelfReference",
            "IFCRELASSIGNSTOPRODUCT" => "IfcRelAssignsToProduct.NoSelfReference",
            _ => "IfcRelAssignsToGroupByFactor.NoSelfReference",
        };
        no_self_reference_named(model, schema, relation, relating_attribute, rule, report);
    }
}

/// `NoSelfReference` where the relating attribute is named per subtype.
///
/// The existing helper hard-codes `RelatingObject`; the assignment family
/// uses four different names for the same role.
fn no_self_reference_named(
    model: &Model,
    schema: &Schema,
    relation: &str,
    relating_attribute: &str,
    rule: &'static str,
    report: &mut Report,
) {
    for (id, entity) in model.of_type(relation) {
        let Some(relating_index) = attribute_index(schema, &entity.type_name, relating_attribute)
        else {
            continue;
        };
        let Some(related_index) = attribute_index(schema, &entity.type_name, "RelatedObjects")
        else {
            continue;
        };
        let Some(relating) = entity.attribute(relating_index).and_then(Value::as_ref_id) else {
            continue;
        };
        let mut includes_self = false;
        if let Some(related) = entity.attribute(related_index) {
            related.for_each_ref(&mut |object| includes_self |= object == relating);
        }
        if includes_self {
            report.push(Finding::error(
                rule,
                attribute_path(id, related_index, "RelatedObjects"),
                format!("related objects contain the relating entity {relating}"),
            ));
        }
    }
}

/// IFC4/IFC4X3 `IfcRelConnectsPathElements` priorities are each in 0..=100.
///
/// The schema states the rule as "the list is empty, OR every member is in
/// range". An empty list is therefore conformant and is not reported; a
/// non-empty list is checked per element so the finding names the offender.
pub fn normalized_connection_priorities(model: &Model, schema: &Schema, report: &mut Report) {
    if !matches!(
        schema.version(),
        Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)
    ) {
        return;
    }
    let checks = [
        (
            "RelatingPriorities",
            "IfcRelConnectsPathElements.NormalizedRelatingPriorities",
        ),
        (
            "RelatedPriorities",
            "IfcRelConnectsPathElements.NormalizedRelatedPriorities",
        ),
    ];
    for (id, entity) in model.of_type("IFCRELCONNECTSPATHELEMENTS") {
        for (attribute, rule) in checks {
            let Some(index) = attribute_index(schema, &entity.type_name, attribute) else {
                continue;
            };
            let Some(value) = entity.attribute(index) else {
                continue;
            };
            let Value::List(items) = value.unwrap_typed() else {
                continue;
            };
            for item in items {
                let Some(priority) = item.unwrap_typed().as_i64() else {
                    continue;
                };
                if !(0..=100).contains(&priority) {
                    report.push(Finding::error(
                        rule,
                        attribute_path(id, index, attribute),
                        format!("priority {priority} is outside the inclusive 0..=100 range"),
                    ));
                }
            }
        }
    }
}

/// IFC4/IFC4X3 `IfcRelSpaceBoundary.CorrectPhysOrVirt`.
///
/// The rule ties the declared physicality to the bounding element's type:
/// PHYSICAL must not be an `IfcVirtualElement`, VIRTUAL must be an
/// `IfcVirtualElement` or an `IfcOpeningElement`, and NOTDEFINED is
/// unconstrained.
///
/// All three concrete subtypes are checked. `of_type` matches exact names,
/// so querying only the supertype would silently skip every 2nd-level
/// boundary -- which is what real BEM exports actually write.
pub fn space_boundary_physicality(model: &Model, schema: &Schema, report: &mut Report) {
    if !matches!(
        schema.version(),
        Some(SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)
    ) {
        return;
    }
    let types = [
        "IFCRELSPACEBOUNDARY",
        "IFCRELSPACEBOUNDARY1STLEVEL",
        "IFCRELSPACEBOUNDARY2NDLEVEL",
    ];
    for boundary_type in types {
        for (id, entity) in model.of_type(boundary_type) {
            check_phys_or_virt(model, schema, id, entity, report);
        }
    }
}

/// One boundary's physicality against its bounding element.
fn check_phys_or_virt(
    model: &Model,
    schema: &Schema,
    id: EntityId,
    entity: &Entity,
    report: &mut Report,
) {
    let Some(physicality_index) =
        attribute_index(schema, &entity.type_name, "PhysicalOrVirtualBoundary")
    else {
        return;
    };
    let Some(element_index) = attribute_index(schema, &entity.type_name, "RelatedBuildingElement")
    else {
        return;
    };
    let Some(declared) = entity
        .attribute(physicality_index)
        .map(Value::unwrap_typed)
        .and_then(|value| match value {
            Value::Enum(member) => Some(member.as_ref()),
            _ => None,
        })
    else {
        return;
    };
    let Some(element) = entity.attribute(element_index).and_then(Value::as_ref_id) else {
        return;
    };
    let Some(target) = model.get(element) else {
        return;
    };
    let is_virtual = schema.is_a(&target.type_name, "IFCVIRTUALELEMENT");
    let is_opening = schema.is_a(&target.type_name, "IFCOPENINGELEMENT");
    let consistent = match declared.to_ascii_uppercase().as_str() {
        "PHYSICAL" => !is_virtual,
        "VIRTUAL" => is_virtual || is_opening,
        _ => true,
    };
    if !consistent {
        report.push(Finding::error(
            "IfcRelSpaceBoundary.CorrectPhysOrVirt",
            attribute_path(id, physicality_index, "PhysicalOrVirtualBoundary"),
            format!(
                "boundary declares {declared} but the related element {element} is {}",
                target.type_name
            ),
        ));
    }
}

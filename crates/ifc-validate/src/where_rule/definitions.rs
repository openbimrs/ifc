//! Rules over the property sets and types an object is defined by.
//!
//! # `UniquePropertySetNames`
//!
//! IFC4 and IFC4X3 state it twice, with the same functions:
//!
//! ```text
//! IfcObject      UniquePropertySetNames : (SIZEOF(IsDefinedBy) = 0)
//!                                         OR IfcUniqueDefinitionNames(IsDefinedBy);
//! IfcTypeObject  UniquePropertySetNames : NOT(EXISTS(HasPropertySets))
//!                                         OR IfcUniquePropertySetNames(HasPropertySets);
//! ```
//!
//! `IfcUniqueDefinitionNames` collects the `RelatingPropertyDefinition` of
//! every relation into a `SET OF IfcPropertySetDefinition`, opening an
//! `IfcPropertySetDefinitionSet`, and `IfcUniquePropertySetNames` requires
//! `SIZEOF(Names) + Unnamed = SIZEOF(Properties)`, where `Names` is the
//! `SET OF IfcLabel` of the `IfcPropertySet` names and `Unnamed` counts
//! every other definition. Both collections are sets: one set attached
//! twice is one member, and the rule is broken exactly when two distinct
//! `IfcPropertySet`s share a name. IFC2X3 states neither rule.
//!
//! A set whose `Name` is unset makes the union indeterminate, but cannot
//! repair a duplicate elsewhere: two sets sharing a name leave `Names` short
//! whatever the unset name is. So a duplicate is reported regardless, and an
//! unset name alone is left to `IfcPropertySet.ExistsName`.
//!
//! # `ApplicableOccurrence`
//!
//! ```text
//! IFC2X3     IfcTypeProduct WR41                 (over ObjectTypeOf[1])
//! IFC4, 4X3  IfcTypeProduct ApplicableOccurrence (over Types[1])
//! ```
//!
//! Both require every object the type is assigned to through its
//! `IfcRelDefinesByType` to be an `IfcProduct`. The inverse is `SET [0:1]`;
//! a file with two relations for one type breaks that bound, and since a
//! SET has no first member, every relation is checked rather than an
//! arbitrary one.
//!
//! The textual `ApplicableOccurrence` and `IfcPropertySetTemplate
//! .ApplicableEntity` are documentation, stated as a WHERE rule by no
//! release, and are not checked here.
//!
//! # Inverses
//!
//! `IsDefinedBy` and `Types` are inverse attributes, which the model does
//! not store. They are rebuilt here from the forward attributes of every
//! relation, which is what the inverse is defined as.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Value};

use super::builtin::{instances, Rule};
use super::operand::Site;
use crate::report::{Finding, Path, Report};

/// `IfcObject.UniquePropertySetNames`.
pub(super) fn object_set_names(rule: &Rule<'_>, report: &mut Report) {
    let mut defined_by: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
    for (id, relation) in instances(rule.model, rule.schema, "IfcRelDefinesByProperties") {
        let site = rule.site(id, relation);
        let (Some(objects), Some(definition)) = (
            site.slot("RelatedObjects", report),
            site.slot("RelatingPropertyDefinition", report),
        ) else {
            continue;
        };
        let Some(objects) = references(&site, objects, "RelatedObjects", report) else {
            continue;
        };
        let Some(definitions) = definitions(&site, definition, report) else {
            continue;
        };
        for object in objects {
            defined_by
                .entry(object)
                .or_default()
                .extend(definitions.iter().copied());
        }
    }
    for (id, _) in rule.instances() {
        if let Some(sets) = defined_by.get(&id) {
            duplicate_names(rule, id, sets, report);
        }
    }
}

/// `IfcTypeObject.UniquePropertySetNames`.
pub(super) fn type_set_names(rule: &Rule<'_>, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let Some(index) = site.slot("HasPropertySets", report) else {
            continue;
        };
        // `NOT(EXISTS(HasPropertySets)) OR ...`: unset satisfies the rule.
        let Some(sets) = references(&site, index, "HasPropertySets", report) else {
            continue;
        };
        duplicate_names(rule, id, &sets.into_iter().collect(), report);
    }
}

/// `IfcTypeProduct.WR41` (IFC2X3) and `IfcTypeProduct.ApplicableOccurrence`
/// (IFC4, IFC4X3): a type product is assigned only to products.
pub(super) fn applicable_occurrence(rule: &Rule<'_>, report: &mut Report) {
    let mut typed: BTreeMap<EntityId, Vec<(Site<'_>, usize, Vec<EntityId>)>> = BTreeMap::new();
    for (id, relation) in instances(rule.model, rule.schema, "IfcRelDefinesByType") {
        let site = rule.site(id, relation);
        let (Some(objects), Some(relating)) = (
            site.slot("RelatedObjects", report),
            site.slot("RelatingType", report),
        ) else {
            continue;
        };
        let Some(relating) = site.reference(relating, "RelatingType", report) else {
            continue;
        };
        let Some(related) = references(&site, objects, "RelatedObjects", report) else {
            continue;
        };
        typed
            .entry(relating)
            .or_default()
            .push((site, objects, related));
    }
    for (id, _) in rule.instances() {
        let Some(relations) = typed.get(&id) else {
            continue;
        };
        for (site, index, related) in relations {
            for &object in related {
                let Some(target) =
                    site.target(rule.model, object, *index, "RelatedObjects", report)
                else {
                    continue;
                };
                if !rule.schema.is_a(&target.type_name, "IfcProduct") {
                    report.push(Finding::error(
                        rule.entry.id,
                        Path::Entity(id),
                        format!(
                            "type {id} is assigned by {} to {object}, a {}, which is not \
                             an IfcProduct",
                            site.id, target.type_name
                        ),
                    ));
                }
            }
        }
    }
}

/// The references an aggregate slot holds, deduplicated in written order.
///
/// `None` when the slot is unset (the caller's rule decides what that
/// means) and, with an evaluation error, when it is not an aggregate of
/// references.
fn references(
    site: &Site<'_>,
    index: usize,
    attribute: &str,
    report: &mut Report,
) -> Option<Vec<EntityId>> {
    let value = site.value(index)?;
    let Value::List(members) = value.unwrap_typed() else {
        site.unreadable(index, attribute, "an aggregate", value, report);
        return None;
    };
    let mut out = Vec::new();
    for member in members {
        let Some(id) = member.unwrap_typed().as_ref_id() else {
            site.unreadable(
                index,
                attribute,
                "an aggregate of entity references",
                member,
                report,
            );
            return None;
        };
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Some(out)
}

/// The property set definitions a relation's `RelatingPropertyDefinition`
/// names: one reference, or an `IfcPropertySetDefinitionSet` of them.
fn definitions(site: &Site<'_>, index: usize, report: &mut Report) -> Option<Vec<EntityId>> {
    const ATTRIBUTE: &str = "RelatingPropertyDefinition";
    let value = site.value(index)?;
    match value.unwrap_typed() {
        Value::Ref(id) => Some(vec![*id]),
        Value::List(_) => references(site, index, ATTRIBUTE, report),
        _ => {
            site.unreadable(
                index,
                ATTRIBUTE,
                "an entity reference or an IfcPropertySetDefinitionSet",
                value,
                report,
            );
            None
        }
    }
}

/// Reports every name that two or more distinct `IfcPropertySet`s among
/// `sets` share, one finding per name, against `owner`.
fn duplicate_names(
    rule: &Rule<'_>,
    owner: EntityId,
    sets: &BTreeSet<EntityId>,
    report: &mut Report,
) {
    let mut named: BTreeMap<&str, Vec<EntityId>> = BTreeMap::new();
    for &set in sets {
        let Some(entity) = rule.model.get(set) else {
            report.push(Finding::evaluation_error(
                rule.entry.id,
                Path::Entity(owner),
                format!(
                    "{owner} is defined by {set}, which the file does not contain, so its \
                     name cannot be compared"
                ),
            ));
            continue;
        };
        // Only an IfcPropertySet contributes a name; every other
        // definition is counted as unnamed, and cannot collide.
        if !rule.schema.is_a(&entity.type_name, "IfcPropertySet") {
            continue;
        }
        let site = rule.site(set, entity);
        let Some(index) = site.slot("Name", report) else {
            continue;
        };
        let Some(value) = site.value(index) else {
            continue;
        };
        let Value::Text(name) = value.unwrap_typed() else {
            site.unreadable(index, "Name", "a string", value, report);
            continue;
        };
        named.entry(name.as_ref()).or_default().push(set);
    }
    for (name, sets) in named {
        if sets.len() < 2 {
            continue;
        }
        let ids: Vec<String> = sets.iter().map(ToString::to_string).collect();
        report.push(Finding::error(
            rule.entry.id,
            Path::Entity(owner),
            format!(
                "{owner} is defined by property sets {} that share the name '{name}'",
                ids.join(", ")
            ),
        ));
    }
}

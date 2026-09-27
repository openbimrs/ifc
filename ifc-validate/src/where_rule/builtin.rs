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
//!
//! # Scope comes from the registry
//!
//! A rule function never names the entity it constrains or the releases it
//! runs under: [`run`] is called only for an entry that
//! [applies](RuleEntry::applies_to) to the schema, and [`instances`] selects
//! the entry's entity *with its subtypes*. An exact-type query here once left
//! `IfcMaterialLayerWithOffsets` and plain `IfcRelAssignsToGroup` unchecked.
//!
//! # When a rule cannot read its input
//!
//! Operands are read through [`Site`], which turns an attribute the tables
//! do not declare, an operand of the wrong shape, or a target the file does
//! not contain into an evaluation-error finding. No rule here skips an
//! instance it applies to without saying so; the one silent case is an
//! unset operand, whose presence `structure` judges.

use std::collections::HashMap;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, SchemaVersion};

use super::operand::Site;
use super::registry::RuleEntry;
use crate::report::{Finding, Path, Report};

/// Evaluates the implemented rule `entry` names.
///
/// Returns `false` when no native implementation carries that id, so the
/// engine and its tests can tell an unwired registry entry from a rule that
/// found nothing.
pub fn run(entry: &RuleEntry, model: &Model, schema: &Schema, report: &mut Report) -> bool {
    let rule = Rule {
        entry,
        model,
        schema,
    };
    match entry.id {
        "global.IfcSingleProjectInstance" => single_project_instance(&rule, report),
        "global.UniqueGlobalId" => unique_global_id(&rule, report),
        "IfcRelDefinesByProperties.NoRelatedTypeObject" => no_related_type_object(&rule, report),
        "IfcExternalReference.WR1" => external_reference_identity(&rule, report),
        "IfcRelSequence.WR1" | "IfcRelSequence.AvoidInconsistentSequence" => {
            sequence_endpoints_differ(&rule, report);
        }
        "IfcRelAggregates.NoSelfReference" | "IfcRelNests.NoSelfReference" => {
            no_self_reference(&rule, "RelatingObject", report);
        }
        "IfcRelAssignsToActor.NoSelfReference" => {
            no_self_reference(&rule, "RelatingActor", report);
        }
        "IfcRelAssignsToProcess.NoSelfReference" => {
            no_self_reference(&rule, "RelatingProcess", report);
        }
        "IfcRelAssignsToProduct.NoSelfReference" => {
            no_self_reference(&rule, "RelatingProduct", report);
        }
        "IfcRelAssignsToGroup.NoSelfReference" => {
            no_self_reference(&rule, "RelatingGroup", report);
        }
        "IfcMaterialLayer.NormalizedPriority" => normalized_material_priority(&rule, report),
        "IfcRelConnectsPathElements.NormalizedRelatingPriorities" => {
            normalized_connection_priorities(&rule, "RelatingPriorities", report);
        }
        "IfcRelConnectsPathElements.NormalizedRelatedPriorities" => {
            normalized_connection_priorities(&rule, "RelatedPriorities", report);
        }
        "IfcRelSpaceBoundary.CorrectPhysOrVirt" => space_boundary_physicality(&rule, report),
        _ => return false,
    }
    true
}

/// One registered rule applied to one model.
struct Rule<'a> {
    entry: &'a RuleEntry,
    model: &'a Model,
    schema: &'a Schema,
}

impl<'a> Rule<'a> {
    /// The operand reader for one instance.
    fn site(&self, id: EntityId, entity: &'a Entity) -> Site<'a> {
        Site {
            rule: self.entry.id,
            id,
            entity,
            schema: self.schema,
        }
    }

    /// Every instance the entry's declaring entity constrains.
    fn instances(&self) -> Vec<(EntityId, &'a Entity)> {
        self.entry
            .entity
            .map(|entity| instances(self.model, self.schema, entity))
            .unwrap_or_default()
    }
}

/// Every instance of `entity` or of a subtype of it, by ascending id.
///
/// EXPRESS WHERE rules are inherited, so a rule declared on an entity binds
/// its subtypes as well. The type names are read from the model's own index
/// and tested once each, rather than testing every record.
pub fn instances<'m>(
    model: &'m Model,
    schema: &Schema,
    entity: &str,
) -> Vec<(EntityId, &'m Entity)> {
    let mut found: Vec<(EntityId, &Entity)> = model
        .type_histogram()
        .into_iter()
        .filter(|(name, _)| schema.is_a(name, entity))
        .flat_map(|(name, _)| model.of_type(name))
        .collect();
    found.sort_by_key(|(id, _)| *id);
    found
}

/// `IfcSingleProjectInstance`: `SIZEOF(IfcProject) <= 1`.
///
/// A global rule, so its path is the file rather than any single entity --
/// no one `IfcProject` is at fault.
fn single_project_instance(rule: &Rule<'_>, report: &mut Report) {
    let projects = instances(rule.model, rule.schema, "IfcProject");
    if projects.len() > 1 {
        let ids: Vec<String> = projects.iter().map(|(id, _)| id.to_string()).collect();
        report.push(Finding::error(
            rule.entry.id,
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
/// error, the repeat is. Deterministic because entries are sorted by id.
fn unique_global_id(rule: &Rule<'_>, report: &mut Report) {
    let mut entries: Vec<(EntityId, usize, &str)> = Vec::new();
    for (id, entity) in instances(rule.model, rule.schema, "IfcRoot") {
        let site = rule.site(id, entity);
        let Some(index) = site.slot("GlobalId", report) else {
            continue;
        };
        let Some(value) = site.value(index) else {
            continue;
        };
        match value.unwrap_typed() {
            Value::Text(guid) => entries.push((id, index, guid.as_ref())),
            _ => site.unreadable(index, "GlobalId", "a string", value, report),
        }
    }
    let mut seen: HashMap<&str, EntityId> = HashMap::new();
    for (id, index, guid) in entries {
        if let Some(first) = seen.get(guid) {
            report.push(Finding::error(
                rule.entry.id,
                Path::Attribute {
                    entity: id,
                    index,
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
/// `RelatedObjects` must contain no `IfcTypeObject`. Type-level property sets
/// travel through `IfcRelDefinesByType` instead.
fn no_related_type_object(rule: &Rule<'_>, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let Some(index) = site.slot("RelatedObjects", report) else {
            continue;
        };
        let Some(related) = site.value(index) else {
            continue;
        };
        let Value::List(members) = related.unwrap_typed() else {
            site.unreadable(index, "RelatedObjects", "an aggregate", related, report);
            continue;
        };
        let mut offenders = Vec::new();
        for member in members {
            let Some(target) = member.unwrap_typed().as_ref_id() else {
                site.unreadable(
                    index,
                    "RelatedObjects",
                    "an aggregate of entity references",
                    member,
                    report,
                );
                continue;
            };
            let Some(object) = site.target(rule.model, target, index, "RelatedObjects", report)
            else {
                continue;
            };
            if rule.schema.is_a(&object.type_name, "IfcTypeObject") {
                offenders.push(target);
            }
        }
        offenders.sort_unstable();
        for offender in offenders {
            report.push(Finding::error(
                site.rule,
                site.path(index, "RelatedObjects"),
                format!(
                    "{offender} is an IfcTypeObject; type property sets \
                     attach through IfcRelDefinesByType"
                ),
            ));
        }
    }
}

/// `IfcExternalReference.WR1`: at least one external identity field exists.
///
/// The operands differ by release, per the bundled EXPRESS: IFC2X3 states
/// `EXISTS(ItemReference) OR EXISTS(Location) OR EXISTS(Name)`, IFC4 and
/// IFC4X3 replaced `ItemReference` with `Identification`.
fn external_reference_identity(rule: &Rule<'_>, report: &mut Report) {
    let operands = if rule.schema.version() == Some(SchemaVersion::Ifc2x3) {
        ["ItemReference", "Location", "Name"]
    } else {
        ["Identification", "Location", "Name"]
    };
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let mut identified = false;
        let mut undeclared = Vec::new();
        for name in operands {
            match site.lookup(name) {
                Some(index) => identified |= site.value(index).is_some(),
                None => undeclared.push(name),
            }
        }
        // One present operand satisfies the disjunction whatever an
        // unresolvable one would say; only an unsatisfied rule with an
        // unresolvable operand is undecided.
        if identified {
            continue;
        }
        if undeclared.is_empty() {
            report.push(Finding::error(
                site.rule,
                Path::Entity(id),
                "external reference has no identification, location, or name",
            ));
        }
        for name in undeclared {
            site.undeclared(name, report);
        }
    }
}

/// Sequence endpoints must refer to different processes.
///
/// IFC2X3 labels the predicate `WR1`, IFC4 on `AvoidInconsistentSequence`;
/// the registry runs whichever the schema declares.
fn sequence_endpoints_differ(rule: &Rule<'_>, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let (Some(relating_index), Some(related_index)) = (
            site.slot("RelatingProcess", report),
            site.slot("RelatedProcess", report),
        ) else {
            continue;
        };
        let endpoints = (
            site.reference(relating_index, "RelatingProcess", report),
            site.reference(related_index, "RelatedProcess", report),
        );
        if let (Some(relating), Some(related)) = endpoints {
            if relating == related {
                report.push(Finding::error(
                    site.rule,
                    site.path(related_index, "RelatedProcess"),
                    format!("sequence endpoints both refer to {related}"),
                ));
            }
        }
    }
}

/// `NoSelfReference`: the relating end must not appear among the related
/// objects.
///
/// The relating attribute is named per relation (`RelatingObject`,
/// `RelatingActor`, ...), so it is a parameter rather than a shared slot.
/// A related member that is not a reference cannot be identical to the
/// relating instance, so it does not make the rule undecidable.
fn no_self_reference(rule: &Rule<'_>, relating_attribute: &str, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let (Some(relating_index), Some(related_index)) = (
            site.slot(relating_attribute, report),
            site.slot("RelatedObjects", report),
        ) else {
            continue;
        };
        let Some(relating) = site.reference(relating_index, relating_attribute, report) else {
            continue;
        };
        let mut includes_self = false;
        if let Some(related) = site.value(related_index) {
            related.for_each_ref(&mut |object| includes_self |= object == relating);
        }
        if includes_self {
            report.push(Finding::error(
                site.rule,
                site.path(related_index, "RelatedObjects"),
                format!("related objects contain the relating entity {relating}"),
            ));
        }
    }
}

/// Material-layer priority, when set, is in the inclusive 0..=100 range.
fn normalized_material_priority(rule: &Rule<'_>, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let Some(index) = site.slot("Priority", report) else {
            continue;
        };
        // `NOT(EXISTS(Priority)) OR ...`: unset satisfies the rule.
        let Some(value) = site.value(index) else {
            continue;
        };
        let Some(priority) = value.unwrap_typed().as_i64() else {
            site.unreadable(index, "Priority", "an integer", value, report);
            continue;
        };
        if !(0..=100).contains(&priority) {
            report.push(Finding::error(
                site.rule,
                site.path(index, "Priority"),
                format!("priority {priority} is outside the inclusive 0..=100 range"),
            ));
        }
    }
}

/// `IfcRelConnectsPathElements` priorities in `attribute` are each in
/// 0..=100.
///
/// The schema states the rule as "the list is empty, OR every member is in
/// range". An empty list is therefore conformant and is not reported; a
/// non-empty list is checked per element so the finding names the offender.
fn normalized_connection_priorities(rule: &Rule<'_>, attribute: &str, report: &mut Report) {
    for (id, entity) in rule.instances() {
        let site = rule.site(id, entity);
        let Some(index) = site.slot(attribute, report) else {
            continue;
        };
        let Some(value) = site.value(index) else {
            continue;
        };
        let Value::List(items) = value.unwrap_typed() else {
            site.unreadable(index, attribute, "an aggregate", value, report);
            continue;
        };
        for item in items {
            let Some(priority) = item.unwrap_typed().as_i64() else {
                site.unreadable(index, attribute, "an aggregate of integers", item, report);
                continue;
            };
            if !(0..=100).contains(&priority) {
                report.push(Finding::error(
                    site.rule,
                    site.path(index, attribute),
                    format!("priority {priority} is outside the inclusive 0..=100 range"),
                ));
            }
        }
    }
}

/// `IfcRelSpaceBoundary.CorrectPhysOrVirt`.
///
/// The rule ties the declared physicality to the bounding element's type:
/// PHYSICAL must not be an `IfcVirtualElement`, VIRTUAL must be an
/// `IfcVirtualElement` or an `IfcOpeningElement`, and NOTDEFINED is
/// unconstrained. The 1st- and 2nd-level subtypes inherit it, and the
/// 2nd-level form is what real BEM exports actually write.
fn space_boundary_physicality(rule: &Rule<'_>, report: &mut Report) {
    for (id, entity) in rule.instances() {
        check_phys_or_virt(rule.model, &rule.site(id, entity), report);
    }
}

/// What the declared physicality demands of the bounding element.
enum Physicality {
    /// Must not be an `IfcVirtualElement`.
    Physical,
    /// Must be an `IfcVirtualElement` or an `IfcOpeningElement`.
    Virtual,
    /// Unconstrained.
    NotDefined,
}

/// One boundary's physicality against its bounding element.
fn check_phys_or_virt(model: &Model, site: &Site<'_>, report: &mut Report) {
    const PHYSICALITY: &str = "PhysicalOrVirtualBoundary";
    const ELEMENT: &str = "RelatedBuildingElement";
    let (Some(physicality_index), Some(element_index)) =
        (site.slot(PHYSICALITY, report), site.slot(ELEMENT, report))
    else {
        return;
    };
    let Some(value) = site.value(physicality_index) else {
        return;
    };
    let declared = match value.unwrap_typed() {
        Value::Enum(member) => member,
        _ => {
            site.unreadable(
                physicality_index,
                PHYSICALITY,
                "an enumeration",
                value,
                report,
            );
            return;
        }
    };
    let physicality = match declared.to_ascii_uppercase().as_str() {
        "PHYSICAL" => Physicality::Physical,
        "VIRTUAL" => Physicality::Virtual,
        "NOTDEFINED" => Physicality::NotDefined,
        _ => {
            report.push(Finding::evaluation_error(
                site.rule,
                site.path(physicality_index, PHYSICALITY),
                format!(
                    "{declared} is not PHYSICAL, VIRTUAL or NOTDEFINED, so the rule \
                     cannot be evaluated"
                ),
            ));
            return;
        }
    };
    if matches!(physicality, Physicality::NotDefined) {
        return;
    }
    let Some(element) = site.reference(element_index, ELEMENT, report) else {
        return;
    };
    let Some(target) = site.target(model, element, element_index, ELEMENT, report) else {
        return;
    };
    let is_virtual = site.schema.is_a(&target.type_name, "IFCVIRTUALELEMENT");
    let is_opening = site.schema.is_a(&target.type_name, "IFCOPENINGELEMENT");
    let consistent = match physicality {
        Physicality::Physical => !is_virtual,
        Physicality::Virtual => is_virtual || is_opening,
        Physicality::NotDefined => true,
    };
    if !consistent {
        report.push(Finding::error(
            site.rule,
            site.path(physicality_index, PHYSICALITY),
            format!(
                "boundary declares {declared} but the related element {element} is {}",
                target.type_name
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::where_rule::registry;

    /// Every entry registered as implemented has a native implementation.
    ///
    /// The registry is a claim; an entry the dispatch table does not know
    /// would run nothing and read as passed.
    #[test]
    fn every_implemented_entry_is_dispatched() {
        let model = Model::new();
        for entry in registry::implemented() {
            let mut report = Report::new();
            assert!(
                run(entry, &model, ifc_schema::ifc4(), &mut report),
                "{} is registered as implemented but nothing runs it",
                entry.id
            );
        }
    }

    /// Instance selection includes subtypes and nothing else.
    #[test]
    fn instances_include_subtypes() {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        let layer = model.push(Entity::new("IFCMATERIALLAYER", Vec::new()));
        let offsets = model.push(Entity::new("IFCMATERIALLAYERWITHOFFSETS", Vec::new()));
        model.push(Entity::new("IFCMATERIAL", Vec::new()));
        let ids: Vec<EntityId> = instances(&model, schema, "IfcMaterialLayer")
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, [layer, offsets]);
    }
}

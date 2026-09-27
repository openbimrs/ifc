//! Which objects carry a templated set, and whether its template admits them.
//!
//! # Carriers
//!
//! A set reaches an occurrence through `IfcRelDefinesByProperties`
//! (`RelatingPropertyDefinition` may be one definition or, from IFC4, an
//! `IfcPropertySetDefinitionSet`), and a type through its own
//! `IfcTypeObject.HasPropertySets`. Both routes are followed; which side an
//! object is on is decided by its entity, not by the route.
//!
//! # `ApplicableEntity`
//!
//! IFC4 ADD2 TC1 `IfcPropertySetTemplate.ApplicableEntity`: "The IFC entity
//! name of the applicable entity using the IFC naming convention, CamelCase
//! with IFC prefix. It can be optionally followed by the predefined type
//! after the separator "/" (forward slash), using upper case. If a
//! performance history object of a particular distribution object is
//! attributes by the property set template, then the entity name (and
//! potentially amended by the predefined type) is expanded by adding
//! '[PerformanceHistory]'. If one property set template is applicable to
//! many type and/or occurrence objects, then those object names should be
//! separate by comma "," forming a comma separated string." Absent, "no
//! instruction is given", so nothing is checked.
//!
//! Where that text leaves the answer open, the reading that reports nothing
//! is taken, or the finding says it cannot decide:
//!
//! - An entry admits subtypes of its entity. The documentation does not say
//!   whether it does; its example lists `IfcWallStandardCase` next to
//!   `IfcWall`, which neither reading contradicts.
//! - An entry admits a type object only if it names a type entity: the
//!   attribute is "the data type of the applicable type or occurrence
//!   object", and the example lists `IfcWallType` separately. `IfcWall` and
//!   `IfcWallType` are unrelated entities, so nothing is inferred between
//!   them.
//! - Entity names and predefined types compare without regard to case, and
//!   blanks around an entry are ignored (the example writes `, `).
//! - An entry qualified by a predefined type admits an object whose own
//!   `PredefinedType` is that constant and excludes one whose
//!   `PredefinedType` is another constant; unset or `NOTDEFINED` cannot be
//!   decided without following the type object.
//! - A `[PerformanceHistory]` entry excludes everything but an
//!   `IfcPerformanceHistory`, and cannot decide for one, since the object it
//!   controls (`IfcRelAssignsToControl`) is not followed.
//! - An entry naming no entity of the declared release, or of another form,
//!   cannot decide.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Value};

use super::super::layout::Layout;
use super::finding::UndecidedReason;
use crate::error::PropertyAnomaly;

/// The objects carrying each set in `templated`, ascending by id.
pub(super) fn carriers(
    model: &Model,
    layout: Layout,
    templated: &BTreeSet<EntityId>,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> BTreeMap<EntityId, BTreeSet<EntityId>> {
    let mut out: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
    let schema = layout.schema();
    let mut relationships = vec!["IFCRELDEFINESBYPROPERTIES"];
    relationships.extend(schema.subtypes("IFCRELDEFINESBYPROPERTIES"));
    for relationship in relationships {
        for &id in model.ids_of_type(relationship) {
            let Some(rel) = model.get(id) else { continue };
            let definitions = definition_refs(layout.get(rel, "RelatingPropertyDefinition"));
            let sets: Vec<EntityId> = definitions
                .into_iter()
                .filter(|set| templated.contains(set))
                .collect();
            if sets.is_empty() {
                continue;
            }
            for object in list_refs(layout.get(rel, "RelatedObjects")) {
                if model.get(object).is_none() {
                    anomalies.push(PropertyAnomaly::MissingObject {
                        relationship: id,
                        object,
                    });
                    continue;
                }
                for set in &sets {
                    out.entry(*set).or_default().insert(object);
                }
            }
        }
    }
    for (type_name, _) in model.type_histogram() {
        if !layout.is_a(type_name, "IFCTYPEOBJECT") {
            continue;
        }
        for &id in model.ids_of_type(type_name) {
            let Some(type_object) = model.get(id) else {
                continue;
            };
            for set in list_refs(layout.get(type_object, "HasPropertySets")) {
                if templated.contains(&set) {
                    out.entry(set).or_default().insert(id);
                }
            }
        }
    }
    out
}

/// `RelatingPropertyDefinition`: one reference, or a (typed) set of them.
fn definition_refs(value: Option<&Value>) -> Vec<EntityId> {
    match value.map(Value::unwrap_typed) {
        Some(Value::Ref(id)) => vec![*id],
        Some(list @ Value::List(_)) => list_refs(Some(list)),
        _ => Vec::new(),
    }
}

fn list_refs(value: Option<&Value>) -> Vec<EntityId> {
    match value {
        Some(Value::List(items)) => items.iter().filter_map(Value::as_ref_id).collect(),
        _ => Vec::new(),
    }
}

/// What an `ApplicableEntity` says about one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Some entry admits it.
    Admits,
    /// Every entry excludes it.
    Excludes,
    /// No entry admits it, and at least one cannot decide.
    Undecided(UndecidedReason),
}

/// Judge `object` against the whole `ApplicableEntity` text.
pub(super) fn applicable(layout: Layout, object: &Entity, applicable_entity: &str) -> Verdict {
    let mut undecided = None;
    for entry in applicable_entity.split(',') {
        match admits(layout, object, entry.trim()) {
            Verdict::Admits => return Verdict::Admits,
            Verdict::Undecided(reason) => {
                undecided.get_or_insert(reason);
            }
            Verdict::Excludes => {}
        }
    }
    undecided.map_or(Verdict::Excludes, Verdict::Undecided)
}

/// One parsed entry: `IfcEntity[/PREDEFINED][[PerformanceHistory]]`.
struct Entry<'a> {
    entity: &'a str,
    predefined: Option<&'a str>,
    performance_history: bool,
}

const PERFORMANCE_HISTORY: &str = "[PerformanceHistory]";

fn parse(entry: &str) -> Option<Entry<'_>> {
    let split = entry.len().checked_sub(PERFORMANCE_HISTORY.len());
    let (body, performance_history) = match split.and_then(|at| entry.split_at_checked(at)) {
        Some((body, suffix)) if suffix.eq_ignore_ascii_case(PERFORMANCE_HISTORY) => (body, true),
        _ => (entry, false),
    };
    let (entity, predefined) = match body.split_once('/') {
        Some((entity, predefined)) => (entity, Some(predefined)),
        None => (body, None),
    };
    let word = |text: &str| {
        !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !word(entity) || predefined.is_some_and(|p| !word(p)) {
        return None;
    }
    Some(Entry {
        entity,
        predefined,
        performance_history,
    })
}

fn admits(layout: Layout, object: &Entity, text: &str) -> Verdict {
    let undecided =
        |reason: fn(Arc<str>) -> UndecidedReason| Verdict::Undecided(reason(text.into()));
    let Some(entry) = parse(text) else {
        return undecided(|entry| UndecidedReason::UnknownApplicableEntity { entry });
    };
    if layout.schema().entity(entry.entity).is_none() {
        return undecided(|entry| UndecidedReason::UnknownApplicableEntity { entry });
    }
    if entry.performance_history {
        return if layout.is_a(&object.type_name, "IFCPERFORMANCEHISTORY") {
            undecided(|entry| UndecidedReason::PerformanceHistory { entry })
        } else {
            Verdict::Excludes
        };
    }
    if !layout.is_a(&object.type_name, entry.entity) {
        return Verdict::Excludes;
    }
    let Some(predefined) = entry.predefined else {
        return Verdict::Admits;
    };
    match layout
        .get(object, "PredefinedType")
        .map(Value::unwrap_typed)
    {
        Some(Value::Enum(stated)) if stated.eq_ignore_ascii_case(predefined) => Verdict::Admits,
        Some(Value::Enum(stated)) if !stated.eq_ignore_ascii_case("NOTDEFINED") => {
            Verdict::Excludes
        }
        _ => undecided(|entry| UndecidedReason::PredefinedTypeUnstated { entry }),
    }
}

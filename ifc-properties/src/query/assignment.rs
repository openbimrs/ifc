//! Resolving which properties apply to an object.
//!
//! # Precedence is a real decision, and it is made explicit
//!
//! An occurrence inherits property sets from its type
//! (`IfcRelDefinesByType` -> `IfcTypeObject.HasPropertySets`) and may carry
//! its own (`IfcRelDefinesByProperties`). When both state a set of the same
//! NAME, the occurrence wins: that is the IFC4 rule, and it is what every
//! authoring tool means by overriding a type default.
//!
//! Nothing about that is inferable from the entity graph -- both routes are
//! just relationships -- so this module states it once, applies it uniformly,
//! and reports which source each surviving set came from. A caller that needs
//! the type's original value can still see it: overridden sets are not
//! discarded, they are recorded as shadowed.
//!
//! # Slots
//!
//! ```text
//! IfcRelDefinesByType  4 = RelatedObjects  5 = RelatingType
//! ```
//!
//! Note this is the OPPOSITE arrangement to `IfcRelDefinesByProperties`,
//! where the definition is slot 5 and the objects slot 4 -- the same two slot
//! numbers carrying different roles.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, Value};

use crate::error::PropertyAnomaly;
use crate::pset::{property_sets_by_object, Attachment, PropertySet};

const REL_TYPE_RELATED_OBJECTS: usize = 4;
const REL_TYPE_RELATING_TYPE: usize = 5;

/// Where a resolved property set came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Stated directly on the occurrence.
    Occurrence,
    /// Inherited from the object's type.
    Type(EntityId),
}

/// A property set that applies to an object, with its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSet {
    /// Where it came from.
    pub source: Source,
    /// The set itself.
    pub set: PropertySet,
    /// A same-named set this one overrode, if any.
    ///
    /// Populated when an occurrence set shadows a type set. Kept rather than
    /// dropped so a caller can explain WHY a value differs from the type
    /// default, which is a question every model checker eventually asks.
    pub shadowed: Option<Box<ResolvedSet>>,
}

/// Resolved property sets per object, after precedence is applied.
pub type ResolvedProperties = BTreeMap<EntityId, Vec<ResolvedSet>>;

/// Every property set applying to every object, precedence applied.
///
/// Sets are ordered by name so output is deterministic regardless of file
/// ordering or hash iteration.
pub fn resolved_properties(model: &Model) -> (ResolvedProperties, Vec<PropertyAnomaly>) {
    let (direct, mut anomalies) = property_sets_by_object(model);
    let types = type_assignments(model, &mut anomalies);

    // Every object that has properties of its own or a type that does.
    let mut objects: Vec<EntityId> = direct.keys().copied().collect();
    objects.extend(types.keys().copied());
    objects.sort_unstable();
    objects.dedup();

    let mut out: BTreeMap<EntityId, Vec<ResolvedSet>> = BTreeMap::new();
    for object in objects {
        // Type sets first, so occurrence sets can overwrite by name.
        let mut by_name: BTreeMap<String, ResolvedSet> = BTreeMap::new();
        if let Some(&type_id) = types.get(&object) {
            for (attachment, set) in direct.get(&type_id).into_iter().flatten() {
                // A type's own sets are what an occurrence inherits. Sets
                // attached to the type by the forbidden relationship are
                // included: the file states them, and the anomaly already
                // records that it should not have.
                let _ = attachment;
                by_name.insert(
                    key(set),
                    ResolvedSet {
                        source: Source::Type(type_id),
                        set: set.clone(),
                        shadowed: None,
                    },
                );
            }
        }
        for (attachment, set) in direct.get(&object).into_iter().flatten() {
            if *attachment != Attachment::Occurrence {
                continue;
            }
            let shadowed = by_name.remove(&key(set)).map(Box::new);
            by_name.insert(
                key(set),
                ResolvedSet {
                    source: Source::Occurrence,
                    set: set.clone(),
                    shadowed,
                },
            );
        }
        if !by_name.is_empty() {
            out.insert(object, by_name.into_values().collect());
        }
    }
    (out, anomalies)
}

/// Property sets applying to one object.
pub fn properties_of(model: &Model, object: EntityId) -> Vec<ResolvedSet> {
    resolved_properties(model)
        .0
        .remove(&object)
        .unwrap_or_default()
}

/// Find a single property by set name and property name.
///
/// Returns the winning value after precedence, so a caller asking for
/// `Pset_WallCommon.IsExternal` gets the occurrence's answer when it has one
/// and the type's otherwise -- which is what the question means.
pub fn property_value<'a>(
    resolved: &'a [ResolvedSet],
    set_name: &str,
    property_name: &str,
) -> Option<&'a crate::pset::Property> {
    resolved
        .iter()
        .find(|r| r.set.name.as_deref() == Some(set_name))
        .and_then(|r| r.set.property(property_name))
}

/// Map each object to its type, via `IfcRelDefinesByType`.
fn type_assignments(
    model: &Model,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> BTreeMap<EntityId, EntityId> {
    let mut out = BTreeMap::new();
    for &id in model.ids_of_type("IFCRELDEFINESBYTYPE") {
        let Some(rel) = model.get(id) else { continue };
        let Some(type_id) = rel.attributes.get(REL_TYPE_RELATING_TYPE).and_then(one_ref) else {
            continue;
        };
        if model.get(type_id).is_none() {
            anomalies.push(PropertyAnomaly::MissingDefinition {
                relationship: id,
                definition: type_id,
            });
            continue;
        }
        for object in rel
            .attributes
            .get(REL_TYPE_RELATED_OBJECTS)
            .and_then(refs)
            .unwrap_or_default()
        {
            if model.get(object).is_none() {
                anomalies.push(PropertyAnomaly::MissingObject {
                    relationship: id,
                    object,
                });
                continue;
            }
            out.insert(object, type_id);
        }
    }
    out
}

/// Key a set by name, falling back to its id when the file omits the name.
///
/// `ExistsName` requires one, so the fallback only fires on malformed files.
/// Keying those by id keeps them distinct instead of collapsing every
/// unnamed set into one bucket.
fn key(set: &PropertySet) -> String {
    match &set.name {
        Some(name) => name.to_string(),
        None => format!("#{}", set.id.0),
    }
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

fn refs(value: &Value) -> Option<Vec<EntityId>> {
    match value {
        Value::List(items) => Some(items.iter().filter_map(one_ref).collect()),
        _ => None,
    }
}

//! The two attachment routes, followed once for every kind of set.
//!
//! Property sets and quantity sets reach an object the same way: by
//! `IfcRelDefinesByProperties` for an occurrence, and by
//! `IfcTypeObject.HasPropertySets` for a type. [`sets_by_object`] walks both
//! routes with a reader for the set kind in question, so the property and
//! quantity views cannot drift apart. See `set.rs` for the slots and the
//! `NoRelatedTypeObject` rule.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, Value};

use crate::error::PropertyAnomaly;
use crate::pset::Attachment;

/// `IfcRelDefinesByProperties`: objects at slot 4, definition at slot 5.
const REL_RELATED_OBJECTS: usize = 4;
const REL_RELATING_DEFINITION: usize = 5;
/// `IfcTypeObject.HasPropertySets`.
const TYPE_HAS_PROPERTY_SETS: usize = 5;

/// Read one set, or `None` when the entity is absent or of another kind.
pub(crate) type Reader<S> = fn(&Model, EntityId) -> Option<(S, Vec<PropertyAnomaly>)>;

/// Sets found on each object, with how each one was attached.
pub(crate) type ByObject<S> = BTreeMap<EntityId, Vec<(Attachment, S)>>;

/// Read each set once, however many objects share it, keeping its anomalies
/// for one report.
struct Cache<S> {
    sets: BTreeMap<EntityId, Option<S>>,
    anomalies: BTreeMap<EntityId, Vec<PropertyAnomaly>>,
}

impl<S: Clone> Cache<S> {
    fn get(&mut self, model: &Model, read: Reader<S>, id: EntityId) -> Option<S> {
        if let Some(set) = self.sets.get(&id) {
            return set.clone();
        }
        let found = read(model, id).map(|(set, anomalies)| {
            self.anomalies.insert(id, anomalies);
            set
        });
        self.sets.insert(id, found.clone());
        found
    }
}

/// Every set `read` accepts, keyed by the object carrying it.
///
/// Both routes are followed. A set reachable by both appears once per object,
/// with `Attachment` recording how it arrived. Per object, sets are ordered
/// by `id_of`. Anomalies report a type object attached by the forbidden
/// relationship, relationships whose targets are missing from the file and,
/// once per set in set-id order, what `read` reported.
pub(crate) fn sets_by_object<S: Clone>(
    model: &Model,
    read: Reader<S>,
    id_of: fn(&S) -> EntityId,
) -> (ByObject<S>, Vec<PropertyAnomaly>) {
    let mut out: ByObject<S> = BTreeMap::new();
    let mut anomalies = Vec::new();
    let mut cache = Cache {
        sets: BTreeMap::new(),
        anomalies: BTreeMap::new(),
    };
    // Which entities are type objects, from the IFC4 table as always, or
    // the newest bundled one in a build without it (#306).
    let (_, schema) = crate::baseline::table(None);

    // Route 1: IfcRelDefinesByProperties, for occurrences.
    for &id in model.ids_of_type("IFCRELDEFINESBYPROPERTIES") {
        let Some(rel) = model.get(id) else { continue };
        let Some(definition) = rel
            .attributes
            .get(REL_RELATING_DEFINITION)
            .and_then(one_ref)
        else {
            // TODO(#630): IFC4+ allows an IfcPropertySetDefinitionSet here.
            continue;
        };
        let Some(set) = cache.get(model, read, definition) else {
            // A definition of another kind is read by its own view; only note
            // one that is not in the file.
            if model.get(definition).is_none() {
                anomalies.push(PropertyAnomaly::MissingDefinition {
                    relationship: id,
                    definition,
                });
            }
            continue;
        };
        for object in rel
            .attributes
            .get(REL_RELATED_OBJECTS)
            .and_then(refs)
            .unwrap_or_default()
        {
            let Some(target) = model.get(object) else {
                anomalies.push(PropertyAnomaly::MissingObject {
                    relationship: id,
                    object,
                });
                continue;
            };
            // The WHERE rule: a type object must not appear here.
            if schema.is_a(&target.type_name.to_ascii_uppercase(), "IFCTYPEOBJECT") {
                anomalies.push(PropertyAnomaly::TypeAttachedByRelationship {
                    relationship: id,
                    type_object: object,
                });
            }
            out.entry(object)
                .or_default()
                .push((Attachment::Occurrence, set.clone()));
        }
    }

    // Route 2: IfcTypeObject.HasPropertySets, a direct attribute.
    for (type_name, _) in model.type_histogram() {
        let upper = type_name.to_ascii_uppercase();
        if !schema.is_a(&upper, "IFCTYPEOBJECT") {
            continue;
        }
        for &id in model.ids_of_type(&upper) {
            let Some(entity) = model.get(id) else {
                continue;
            };
            for set_id in entity
                .attributes
                .get(TYPE_HAS_PROPERTY_SETS)
                .and_then(refs)
                .unwrap_or_default()
            {
                if let Some(set) = cache.get(model, read, set_id) {
                    out.entry(id).or_default().push((Attachment::Type, set));
                }
            }
        }
    }

    for sets in out.values_mut() {
        sets.sort_by_key(|(_, set)| id_of(set));
    }
    // Member anomalies, once per set and in set-id order.
    for found in cache.anomalies.into_values() {
        anomalies.extend(found);
    }
    (out, anomalies)
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

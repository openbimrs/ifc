//! `IfcPropertySet` and how it reaches an object.
//!
//! # Two attachment routes, and one of them is forbidden for types
//!
//! ```text
//! IfcRelDefinesByProperties  4 = RelatedObjects  5 = RelatingPropertyDefinition
//! IfcTypeObject              5 = HasPropertySets          (a direct attribute)
//! ```
//!
//! `IfcRelDefinesByProperties` carries a WHERE rule:
//!
//! ```text
//! NoRelatedTypeObject : SIZEOF(QUERY(Types <* RelatedObjects |
//!                       'IFC4.IFCTYPEOBJECT' IN TYPEOF(Types))) = 0;
//! ```
//!
//! A type object may NOT be given properties by that relationship: it holds
//! them in its own `HasPropertySets` attribute instead. A reader that only
//! follows the relationship finds no type properties at all, and one that
//! accepts types through it will happily read malformed files without
//! comment. Both routes are read here, and the forbidden combination is
//! reported.

use std::collections::BTreeMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};
use ifc_schema::ifc4;

use crate::error::PropertyAnomaly;
use crate::pset::scalar::{property, Property};

/// `IfcRelDefinesByProperties`: objects at slot 4, definition at slot 5.
const REL_RELATED_OBJECTS: usize = 4;
const REL_RELATING_DEFINITION: usize = 5;
/// `IfcTypeObject.HasPropertySets`.
const TYPE_HAS_PROPERTY_SETS: usize = 5;
/// `IfcPropertySet.HasProperties`.
const SET_HAS_PROPERTIES: usize = 4;
/// `IfcRoot.Name` / `Description`.
const ROOT_NAME: usize = 2;
const ROOT_DESCRIPTION: usize = 3;

/// How a property set reached the object that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attachment {
    /// Attached to an occurrence by `IfcRelDefinesByProperties`.
    Occurrence,
    /// Held directly by an `IfcTypeObject` in `HasPropertySets`.
    Type,
}

/// An `IfcPropertySet` with its properties resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySet {
    /// The `IfcPropertySet` entity.
    pub id: EntityId,
    /// `Name`. The schema requires it (`ExistsName`).
    pub name: Option<Arc<str>>,
    /// `Description`, when stated.
    pub description: Option<Arc<str>>,
    /// Properties in file order.
    pub properties: Vec<Property>,
}

impl PropertySet {
    /// Look up a property by name.
    ///
    /// The schema requires unique property names within a set
    /// (`UniquePropertyNames`), so the first match is the only match in a
    /// well-formed file.
    pub fn property(&self, name: &str) -> Option<&Property> {
        self.properties
            .iter()
            .find(|p| p.name.as_deref() == Some(name))
    }
}

/// Read one `IfcPropertySet` by id, resolving its properties.
///
/// Returns `None` when the entity is absent or is not a property set. Other
/// `IfcPropertySetDefinition` subtypes (quantity sets, predefined sets) are
/// deliberately excluded: they are not `IfcPropertySet` and have their own
/// attribute layouts.
pub fn property_set(model: &Model, id: EntityId) -> Option<PropertySet> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCPROPERTYSET") {
        return None;
    }
    let properties = entity
        .attributes
        .get(SET_HAS_PROPERTIES)
        .and_then(refs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| property(model, p))
        .collect();
    Some(PropertySet {
        id,
        name: entity.attributes.get(ROOT_NAME).and_then(text),
        description: entity.attributes.get(ROOT_DESCRIPTION).and_then(text),
        properties,
    })
}

/// Property sets found on each object, with how each one was attached.
pub type AttachedSets = BTreeMap<EntityId, Vec<(Attachment, PropertySet)>>;

/// Every property set in the file, keyed by the object carrying it.
///
/// Both routes are followed. A set reachable by both appears once per object,
/// with `Attachment` recording how it arrived: precedence is a caller
/// decision, so the reader must not collapse the distinction here.
///
/// Anomalies report a type object attached by the forbidden relationship, and
/// relationships whose targets are missing from the file.
pub fn property_sets_by_object(model: &Model) -> (AttachedSets, Vec<PropertyAnomaly>) {
    let mut out: AttachedSets = BTreeMap::new();
    let mut anomalies = Vec::new();
    let schema = ifc4();

    // Route 1: IfcRelDefinesByProperties, for occurrences.
    for &id in model.ids_of_type("IFCRELDEFINESBYPROPERTIES") {
        let Some(rel) = model.get(id) else { continue };
        let Some(definition) = rel
            .attributes
            .get(REL_RELATING_DEFINITION)
            .and_then(one_ref)
        else {
            continue;
        };
        let Some(set) = property_set(model, definition) else {
            // Quantity sets travel this relationship too and are read by the
            // quantity module; only note a definition that is not in the file.
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
                if let Some(set) = property_set(model, set_id) {
                    out.entry(id).or_default().push((Attachment::Type, set));
                }
            }
        }
    }

    for sets in out.values_mut() {
        sets.sort_by_key(|(_, set)| set.id);
    }
    (out, anomalies)
}

fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
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

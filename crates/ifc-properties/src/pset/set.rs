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

use crate::error::PropertyAnomaly;
use crate::nesting::Nesting;
use crate::pset::attach::sets_by_object;
use crate::pset::scalar::{read_property, Property};

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
#[non_exhaustive]
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
    /// well-formed file. In a malformed one this is the first in
    /// `HasProperties` order, and
    /// [`property_sets_by_object`] reports the others as
    /// [`PropertyAnomaly::DuplicatePropertyName`].
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
///
/// Members this reader cannot resolve (absent ids, cyclic or over-deep
/// complex properties) are left out without saying so; use
/// [`property_set_checked`] to have each one reported.
pub fn property_set(model: &Model, id: EntityId) -> Option<PropertySet> {
    property_set_checked(model, id).map(|(set, _)| set)
}

/// Read one `IfcPropertySet` by id, reporting every member left unread.
///
/// The same value as [`property_set`], with a
/// [`PropertyAnomaly::MissingMember`] for each `HasProperties` id absent
/// from the file, a [`PropertyAnomaly::MemberNotReference`] for each item
/// that is not an entity reference, a [`PropertyAnomaly::DuplicateMember`]
/// for each member listed again (read once), and the nesting anomalies
/// described on
/// [`property_checked`](crate::property_checked) for its complex properties.
pub fn property_set_checked(
    model: &Model,
    id: EntityId,
) -> Option<(PropertySet, Vec<PropertyAnomaly>)> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCPROPERTYSET") {
        return None;
    }
    let mut anomalies = Vec::new();
    let mut nesting = Nesting::new(&mut anomalies);
    let mut properties = Vec::new();
    let members = nesting.members(
        id,
        "HasProperties",
        entity.attributes.get(SET_HAS_PROPERTIES),
    );
    for member in members {
        if !nesting.admit(model, id, member) {
            continue;
        }
        if let Some(property) = read_property(model, member, &mut nesting) {
            properties.push(property);
        }
    }
    let set = PropertySet {
        id,
        name: entity.attributes.get(ROOT_NAME).and_then(text),
        description: entity.attributes.get(ROOT_DESCRIPTION).and_then(text),
        properties,
    };
    Some((set, anomalies))
}

/// Property sets found on each object, with how each one was attached.
pub type AttachedSets = BTreeMap<EntityId, Vec<(Attachment, PropertySet)>>;

/// Every property set in the file, keyed by the object carrying it.
///
/// Both routes are followed. A set reachable by both appears once per object,
/// with `Attachment` recording how it arrived: precedence is a caller
/// decision, so the reader must not collapse the distinction here.
///
/// Anomalies report a type object attached by the forbidden relationship,
/// relationships whose targets are missing from the file, and, once per set,
/// the members [`property_set_checked`] could not resolve.
pub fn property_sets_by_object(model: &Model) -> (AttachedSets, Vec<PropertyAnomaly>) {
    let (out, mut anomalies) =
        sets_by_object(model, property_set_checked, |set: &PropertySet| set.id);
    // A set shared by many objects is checked once.
    let mut unique: BTreeMap<EntityId, &PropertySet> = BTreeMap::new();
    for (_, set) in out.values().flatten() {
        unique.entry(set.id).or_insert(set);
    }
    for set in unique.values() {
        duplicate_property_names(set, &mut anomalies);
    }
    (out, anomalies)
}

/// `UniquePropertyNames` (IFC4) / `WR32` (IFC2X3): report each property whose
/// name an earlier one in the same set already has.
fn duplicate_property_names(set: &PropertySet, anomalies: &mut Vec<PropertyAnomaly>) {
    let mut first: BTreeMap<&str, EntityId> = BTreeMap::new();
    for property in &set.properties {
        let Some(name) = property.name.as_deref() else {
            continue;
        };
        match first.get(name) {
            Some(&kept) => anomalies.push(PropertyAnomaly::DuplicatePropertyName {
                set: set.id,
                kept,
                rejected: property.id,
            }),
            None => {
                first.insert(name, property.id);
            }
        }
    }
}

fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

//! `IfcZone` and what it is allowed to contain.
//!
//! # WR1 is a real constraint, not a convention
//!
//! `IfcZone` carries a WHERE rule in IFC4 restricting its members to
//! `IfcZone`, `IfcSpace` and `IfcSpatialZone` -- nothing else. A zone
//! grouping a pump is not a stylistic choice, it is an invalid file.
//!
//! The rule is enforced as a REPORTED anomaly rather than a hard error: a
//! file with one bad member still has a usable zone structure, and refusing
//! the whole read would lose the valid members too.
//!
//! # Zones are systems
//!
//! `IfcZone -> IfcSystem -> IfcGroup`, so `systems()` already returns zones.
//! This module adds what is specific to them: the member restriction, the
//! `LongName` at slot 5, and the spatial elements they cover.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};
use ifc_schema::ifc4;

use crate::error::SystemAnomaly;

/// Attribute slots. `IfcZone` adds `LongName` at 5, after the four
/// `IfcObject` attributes; it has no placement or representation because a
/// zone is a grouping, not a product.
mod slot {
    pub const LONG_NAME: usize = 5;
    /// `IfcRelAssignsToGroup`: members at 4, group at 6 (5 is
    /// `RelatedObjectsType`).
    pub const ASSIGNS_MEMBERS: usize = 4;
    pub const ASSIGNS_GROUP: usize = 6;
}

/// The types WR1 permits inside an `IfcZone`.
///
/// Checked by schema ancestry, not string equality: a subtype of `IfcSpace`
/// is still a space, and comparing type names alone would reject it.
const ZONE_MEMBER_TYPES: [&str; 3] = ["IFCZONE", "IFCSPACE", "IFCSPATIALZONE"];

/// A zone: a grouping of spatial elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    /// Entity id of the `IfcZone` itself.
    pub id: EntityId,
    /// `Name`, if the file states one.
    pub name: Option<String>,
    /// `LongName` (slot 5), the descriptive name.
    pub long_name: Option<String>,
    /// Members that satisfy WR1, ascending by id.
    ///
    /// Members violating WR1 are NOT here: they are reported as anomalies, so
    /// this list is always a valid zone content set.
    pub members: Vec<EntityId>,
}

fn refs(value: Option<&Value>) -> Vec<EntityId> {
    match value {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect(),
        Some(Value::Ref(id)) => vec![*id],
        _ => Vec::new(),
    }
}

fn text(model: &Model, id: EntityId, slot: usize) -> Option<String> {
    match model.get(id)?.attributes.get(slot)? {
        Value::Text(t) => Some(t.to_string()),
        _ => None,
    }
}

/// Every `IfcZone` in the file, with WR1-valid members resolved.
///
/// Zones are found by schema ancestry so that any future subtype is included
/// automatically, consistent with how systems and ports are discovered.
pub fn zones(model: &Model) -> (Vec<Zone>, Vec<SystemAnomaly>) {
    let mut anomalies = Vec::new();

    let schema = ifc4();
    let mut zone_ids = BTreeSet::new();
    for (type_name, _) in model.type_histogram() {
        if schema.is_a(type_name, "IFCZONE") {
            zone_ids.extend(model.ids_of_type(type_name).iter().copied());
        }
    }

    // Members, gathered per zone and filtered by WR1.
    let mut members: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    for &relation in model.ids_of_type("IFCRELASSIGNSTOGROUP") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let group = match entity.attributes.get(slot::ASSIGNS_GROUP) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if !zone_ids.contains(&group) {
            continue;
        }
        for member in refs(entity.attributes.get(slot::ASSIGNS_MEMBERS)) {
            let Some(member_entity) = model.get(member) else {
                anomalies.push(SystemAnomaly::Dangling {
                    relation,
                    missing: member,
                });
                continue;
            };
            let type_name = member_entity.type_name.to_ascii_uppercase();
            let permitted = ZONE_MEMBER_TYPES
                .iter()
                .any(|allowed| schema.is_a(&type_name, allowed));
            if permitted {
                members.entry(group).or_default().push(member);
            } else {
                // WR1 violation: reported, and excluded from members so a
                // caller iterating a zone never sees a pump in a room list.
                anomalies.push(SystemAnomaly::ZoneMemberNotSpatial {
                    relation,
                    zone: group,
                    member,
                    type_name: member_entity.type_name.to_string(),
                });
            }
        }
    }

    let zones = zone_ids
        .into_iter()
        .map(|id| {
            let mut member_ids = members.remove(&id).unwrap_or_default();
            member_ids.sort_unstable();
            member_ids.dedup();
            Zone {
                id,
                name: text(model, id, 2),
                long_name: text(model, id, slot::LONG_NAME),
                members: member_ids,
            }
        })
        .collect();

    (zones, anomalies)
}

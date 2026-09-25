//! `IfcZone` and what it is allowed to contain.
//!
//! # WR1 is a real constraint, not a convention
//!
//! `IfcZone` carries a WHERE rule restricting its members to `IfcZone`,
//! `IfcSpace`, and in IFC4 also `IfcSpatialZone` -- nothing else. A zone
//! grouping a pump is not a stylistic choice, it is an invalid file.
//!
//! The rule is enforced as a REPORTED anomaly rather than a hard error: a
//! file with one bad member still has a usable zone structure, and refusing
//! the whole read would lose the valid members too.
//!
//! # Zones are systems in IFC4 only
//!
//! IFC4 has `IfcZone -> IfcSystem -> IfcGroup`, so `systems()` returns zones
//! there. IFC2X3 has `IfcZone -> IfcGroup`, so it does not. This module adds
//! what is specific to zones: the member restriction, the IFC4 `LongName` at
//! slot 5, and the spatial elements they cover.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};

use crate::error::{NotInSchema, SchemaGap, SystemAnomaly};
use crate::release::{self, Release};

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
///
/// Reads against the release the model's `FILE_SCHEMA` header declares.
/// WR1 differs by release: IFC4 admits `IfcZone`, `IfcSpace` and
/// `IfcSpatialZone`; IFC2X3 admits only `IfcZone` and `IfcSpace`, because
/// it has no `IfcSpatialZone`. Membership is checked by ancestry in the
/// declared table, so an `IfcSpatialZone` in an IFC2X3 file is reported as
/// `ZoneMemberNotSpatial`. `long_name` is `None` for every zone under
/// IFC2X3, whose `IfcZone` has no `LongName` slot (issue #52). This bulk
/// reader cannot tell that apart from a file that left the slot empty,
/// because `Zone::long_name` predates #52 and stays `Option<String>`. Use
/// [`long_name_of`] when that distinction matters.
pub fn zones(model: &Model) -> (Vec<Zone>, Vec<SystemAnomaly>) {
    let release = release::resolve_or_ifc4(model);
    let mut anomalies = Vec::new();

    let mut zone_ids = BTreeSet::new();
    for (type_name, _) in model.type_histogram() {
        if release.is_a(type_name, "IFCZONE") {
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
                .any(|allowed| release.is_a(&type_name, allowed));
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
                // Under IFC2X3 this is unconditionally None: that release's
                // IfcZone has no LongName slot (`release.has_slot` below is
                // false), so no attempt is made to read slot 5 at all. See
                // long_name_of for a caller that needs to tell that apart
                // from an authored-empty LongName.
                long_name: if release.has_slot("IFCZONE", slot::LONG_NAME) {
                    text(model, id, slot::LONG_NAME)
                } else {
                    None
                },
                members: member_ids,
            }
        })
        .collect();

    (zones, anomalies)
}

/// `LongName` of a single `IfcZone`, distinguishing "not authored" from
/// "this release has no such attribute".
///
/// [`zones`] cannot make this distinction: its `Zone::long_name` field is
/// `Option<String>` and predates #52, so both cases collapse to `None`
/// there. IFC2X3's `IfcZone` has no `LongName` slot at all (it is a plain
/// five-attribute `IfcGroup` subtype), unlike IFC4's, which adds `LongName`
/// as its sixth. Reading it under IFC2X3 is therefore not "the file left it
/// blank" -- there is no slot to have left blank -- and a caller that needs
/// to tell the two apart should use this accessor instead of `Zone::long_name`.
///
/// # Errors
///
/// [`SchemaGap::Schema`] if the model's `FILE_SCHEMA` does not resolve to
/// IFC2X3 or IFC4 (see [`crate::schema_of`]). [`SchemaGap::NotInSchema`] if
/// the resolved release does not declare `LongName` for `IfcZone` (IFC2X3).
pub fn long_name_of(model: &Model, zone: EntityId) -> Result<Option<String>, SchemaGap> {
    let release: Release = release::resolve(model)?;
    if !release.has_slot("IFCZONE", slot::LONG_NAME) {
        return Err(SchemaGap::NotInSchema(NotInSchema {
            entity: zone,
            schema: release.version,
        }));
    }
    Ok(text(model, zone, slot::LONG_NAME))
}

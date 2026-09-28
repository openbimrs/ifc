//! `IfcZone` and what it is allowed to contain.
//!
//! # WR1 is a real constraint, not a convention
//!
//! `IfcZone` carries a WHERE rule restricting its members to `IfcZone`,
//! `IfcSpace`, and from IFC4 also `IfcSpatialZone` -- nothing else. A zone
//! grouping a pump is not a stylistic choice, it is an invalid file.
//!
//! The rule is enforced as a REPORTED anomaly rather than a hard error: a
//! file with one bad member still has a usable zone structure, and refusing
//! the whole read would lose the valid members too.
//!
//! # Zones are systems from IFC4 on
//!
//! IFC4 and IFC4X3 have `IfcZone -> IfcSystem -> IfcGroup`, so `systems()`
//! returns zones there. IFC2X3 has `IfcZone -> IfcGroup`, so it does not.
//! This module adds what is specific to zones: the member restriction, the
//! `LongName` (IFC4 onwards), and the spatial elements they cover.
//!
//! # Every slot is read by name in the declared release
//!
//! ```text
//! IfcZone               IFC2X3  GlobalId OwnerHistory Name Description ObjectType
//!                       IFC4    ... ObjectType LongName          (IFC4X3 the same)
//! IfcRelAssignsToGroup  all     GlobalId OwnerHistory Name Description
//!                               RelatedObjects RelatedObjectsType RelatingGroup
//! ```
//!
//! IFC4X3 ADD2 retypes `IfcRelAssigns.RelatedObjectsType` as
//! `IfcStrippedOptional` but keeps its position, so `RelatingGroup` stays the
//! seventh attribute. Positions come from the table, not from constants.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};

use crate::error::{NotInSchema, SchemaGap, SchemaResolutionError, SystemAnomaly};
use crate::release::{self, Release};

const ZONE: &str = "IFCZONE";
const ASSIGNS: &str = "IFCRELASSIGNSTOGROUP";

/// The types WR1 permits inside an `IfcZone`.
///
/// Checked by schema ancestry, not string equality: a subtype of `IfcSpace`
/// is still a space, and comparing type names alone would reject it.
const ZONE_MEMBER_TYPES: [&str; 3] = ["IFCZONE", "IFCSPACE", "IFCSPATIALZONE"];

/// A zone: a grouping of spatial elements.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Zone {
    /// Entity id of the `IfcZone` itself.
    pub id: EntityId,
    /// `Name`, if the file states one.
    pub name: Option<String>,
    /// `LongName`, the descriptive name (IFC4 onwards).
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

/// The text at the `IfcZone` `attribute` of `id`, read by name in
/// `release`; `None` when unset, not text, or not declared by the release.
/// A subtype keeps its inherited attributes first, so `IfcZone`'s position
/// holds for it too.
fn text(model: &Model, release: Release, id: EntityId, attribute: &str) -> Option<String> {
    let slot = release.slot(ZONE, attribute)?;
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
/// Reads against the release the model's `FILE_SCHEMA` header declares:
/// IFC2X3, IFC4 or IFC4X3 (#194), every slot by attribute name. WR1
/// differs by release: IFC4 and IFC4X3 admit `IfcZone`, `IfcSpace` and
/// `IfcSpatialZone`; IFC2X3 admits only `IfcZone` and `IfcSpace`, because
/// it has no `IfcSpatialZone`. Membership is checked by ancestry in the
/// declared table, so an `IfcSpatialZone` in an IFC2X3 file is reported as
/// `ZoneMemberNotSpatial`. `long_name` is `None` for every zone under
/// IFC2X3, whose `IfcZone` has no `LongName` (issue #52). This bulk reader
/// cannot tell that apart from a file that left it empty, because
/// `Zone::long_name` predates #52 and stays `Option<String>`. Use
/// [`long_name_of`] when that distinction matters.
///
/// # Errors
///
/// [`SchemaResolutionError`] when the model's `FILE_SCHEMA` binds no release
/// this crate is verified for (see [`crate::schema_of`]): no schema,
/// several, or any release but IFC2X3, IFC4 and IFC4X3. None is read as
/// IFC4.
pub fn zones(model: &Model) -> Result<(Vec<Zone>, Vec<SystemAnomaly>), SchemaResolutionError> {
    Ok(zones_in(model, release::resolve(model)?))
}

fn zones_in(model: &Model, release: Release) -> (Vec<Zone>, Vec<SystemAnomaly>) {
    let mut anomalies = Vec::new();

    let mut zone_ids = BTreeSet::new();
    for (type_name, _) in model.type_histogram() {
        if release.is_a(type_name, ZONE) {
            zone_ids.extend(model.ids_of_type(type_name).iter().copied());
        }
    }

    // Members, gathered per zone and filtered by WR1.
    let mut members: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    // Every bundled release declares both; `zone_slots_resolve_by_name`
    // pins them per release.
    let group_slot = release
        .slot(ASSIGNS, "RelatingGroup")
        .expect("every bundled release declares IfcRelAssignsToGroup.RelatingGroup");
    let members_slot = release
        .slot(ASSIGNS, "RelatedObjects")
        .expect("every bundled release declares IfcRelAssigns.RelatedObjects");
    for &relation in model.ids_of_type(ASSIGNS) {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let group = match entity.attributes.get(group_slot) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if !zone_ids.contains(&group) {
            continue;
        }
        for member in refs(entity.attributes.get(members_slot)) {
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
                name: text(model, release, id, "Name"),
                // Under IFC2X3 this is unconditionally None: that release's
                // IfcZone declares no LongName, so no slot is read at all.
                // See long_name_of for a caller that needs to tell that apart
                // from an authored-empty LongName.
                long_name: text(model, release, id, "LongName"),
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
/// there. IFC2X3's `IfcZone` has no `LongName` at all (it is a plain
/// five-attribute `IfcGroup` subtype), unlike IFC4's and IFC4X3's, which add
/// `LongName` as the sixth. Reading it under IFC2X3 is therefore not "the
/// file left it blank" -- there is no slot to have left blank -- and a
/// caller that needs to tell the two apart should use this accessor instead
/// of `Zone::long_name`.
///
/// # Errors
///
/// [`SchemaGap::Schema`] if the model's `FILE_SCHEMA` does not resolve to
/// IFC2X3, IFC4 or IFC4X3. [`SchemaGap::NotInSchema`] if the resolved
/// release does not declare `LongName` for `IfcZone` (IFC2X3).
pub fn long_name_of(model: &Model, zone: EntityId) -> Result<Option<String>, SchemaGap> {
    let release: Release = release::resolve(model)?;
    if release.slot(ZONE, "LongName").is_none() {
        return Err(SchemaGap::NotInSchema(NotInSchema {
            entity: zone,
            schema: release.version,
        }));
    }
    Ok(text(model, release, zone, "LongName"))
}

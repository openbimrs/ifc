//! `IfcSystem` and group semantics.
//!
//! # The slot trap
//!
//! Membership and service use different attribute layouts, and neither is
//! guessable from the other:
//!
//! ```text
//! IfcRelAssignsToGroup       4 = RelatedObjects   6 = RelatingGroup
//! IfcRelServicesBuildings    4 = RelatingSystem   5 = RelatedBuildings
//! ```
//!
//! `IfcRelAssignsToGroup` also carries `RelatedObjectsType` at slot 5, so the
//! group is at 6 and NOT at 5 where every other `IfcRel*` in this crate puts
//! its relating end. Reading slot 5 yields an enumeration, not a reference,
//! and a membership silently vanishes.
//!
//! # Distribution-system attributes are read by name
//!
//! ```text
//! IfcDistributionSystem  IFC4, IFC4X3  ... ObjectType LongName PredefinedType
//! ```
//!
//! `LongName` and `PredefinedType` (`IfcDistributionSystemEnum`) are
//! resolved by attribute name in the declared release's table, so an
//! `IfcDistributionCircuit` (which adds nothing) reads them from its
//! inherited positions. IFC2X3 declares no `IfcDistributionSystem`, so there
//! the fields are `None` for every system, as they are for any system that is
//! not an `IfcDistributionSystem`.

use ifc_model::{EntityId, Model, Value};

use crate::error::{SchemaResolutionError, SystemAnomaly};
use crate::release::{self, Release};

/// Attribute slots, named so a misread is a compile error rather than a
/// silently empty result.
pub(crate) mod slot {
    /// `IfcRelAssignsToGroup.RelatedObjects`.
    pub const ASSIGNS_RELATED: usize = 4;
    /// `IfcRelAssignsToGroup.RelatingGroup` -- 6, not 5.
    pub const ASSIGNS_GROUP: usize = 6;
    /// `IfcRoot.Name`.
    pub const NAME: usize = 2;
}

/// A system as the file states it, with its members resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct System {
    /// The `IfcSystem` (or subtype) entity.
    pub id: EntityId,
    /// Declared type, e.g. `IFCDISTRIBUTIONSYSTEM`.
    pub type_name: String,
    /// `Name`, when present.
    pub name: Option<String>,
    /// `IfcDistributionSystem.LongName`, when present (IFC4, IFC4X3).
    ///
    /// `None` when the file leaves it empty, and for every system that is
    /// not an `IfcDistributionSystem` or subtype (a plain `IfcSystem`, a zone,
    /// any IFC2X3 system), because only that type is read here.
    pub long_name: Option<String>,
    /// `IfcDistributionSystem.PredefinedType`: the `IfcDistributionSystemEnum`
    /// token as the file states it, without dots (e.g. `HEATING`).
    ///
    /// `None` under the same conditions as [`System::long_name`].
    pub predefined_type: Option<String>,
    /// Members, in file order.
    ///
    /// Order is preserved because IFC states no ordering and re-sorting would
    /// invent one; a caller comparing two exports needs the file's own order.
    pub members: Vec<EntityId>,
}

fn text(model: &Model, id: EntityId, slot: usize) -> Option<String> {
    match model.get(id)?.attributes.get(slot)? {
        Value::Text(t) => Some(t.to_string()),
        _ => None,
    }
}

const DISTRIBUTION_SYSTEM: &str = "IFCDISTRIBUTIONSYSTEM";

/// `attribute` of `IfcDistributionSystem` on `id`, read by name in `release`.
///
/// `None` when `type_name` is not an `IfcDistributionSystem` (or subtype)
/// under the release, or when the release does not declare the attribute. A
/// subtype keeps its inherited attributes first, so the supertype's position
/// holds for it.
fn distribution_attribute<'m>(
    model: &'m Model,
    release: Release,
    id: EntityId,
    type_name: &str,
    attribute: &str,
) -> Option<&'m Value> {
    if !release.is_a(type_name, DISTRIBUTION_SYSTEM) {
        return None;
    }
    let slot = release.slot(DISTRIBUTION_SYSTEM, attribute)?;
    model.get(id)?.attributes.get(slot)
}

/// `LongName` (an `IfcLabel`); anything but text is not a label.
fn long_name(model: &Model, release: Release, id: EntityId, type_name: &str) -> Option<String> {
    match distribution_attribute(model, release, id, type_name, "LongName")? {
        Value::Text(t) => Some(t.to_string()),
        _ => None,
    }
}

/// `PredefinedType` (an `IfcDistributionSystemEnum`); anything but an
/// enumeration token is not one.
fn predefined_type(
    model: &Model,
    release: Release,
    id: EntityId,
    type_name: &str,
) -> Option<String> {
    match distribution_attribute(model, release, id, type_name, "PredefinedType")? {
        Value::Enum(token) => Some(token.to_string()),
        _ => None,
    }
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

/// Every system in the file, with members resolved and anomalies reported.
///
/// Systems are found by type, not by walking memberships: a file may declare
/// a system that nothing is assigned to yet, and dropping it would understate
/// the model. Subtypes are included, so `IfcDistributionSystem` and
/// `IfcBuildingSystem` are both found in IFC4, and `IfcElectricalCircuit` is
/// found in IFC2X3 -- but `IfcZone` is NOT, because it subtypes `IfcGroup`
/// rather than `IfcSystem` in IFC2X3 (issue #52).
///
/// Reads against the release the model's `FILE_SCHEMA` header declares:
/// IFC2X3, IFC4 or IFC4X3.
///
/// # Errors
///
/// [`SchemaResolutionError`] when the model's `FILE_SCHEMA` binds no release
/// this crate is verified for (see [`crate::schema_of`]). Nothing is read
/// against a release the file did not declare.
pub fn systems(model: &Model) -> Result<(Vec<System>, Vec<SystemAnomaly>), SchemaResolutionError> {
    let release = release::resolve(model)?;
    let mut anomalies = Vec::new();

    // Membership is stated by the relationship, not the system, so index the
    // relationships once instead of rescanning per system.
    let mut members: std::collections::BTreeMap<EntityId, Vec<EntityId>> =
        std::collections::BTreeMap::new();

    for &relation in model.ids_of_type("IFCRELASSIGNSTOGROUP") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let group = match entity.attributes.get(slot::ASSIGNS_GROUP) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        let Some(group_entity) = model.get(group) else {
            anomalies.push(SystemAnomaly::Dangling {
                relation,
                missing: group,
            });
            continue;
        };
        // The relationship is shared with every group kind; only systems are
        // this crate's concern, and the rest are reported rather than dropped.
        if !release.is_a(&group_entity.type_name.to_ascii_uppercase(), "IFCSYSTEM") {
            anomalies.push(SystemAnomaly::NotASystem {
                relation,
                group,
                // Upper-cased: a STEP file writes IFCINVENTORY while an
                // in-memory model may carry IfcInventory, and a caller
                // matching on this string must not have to know which.
                type_name: group_entity.type_name.to_ascii_uppercase(),
            });
            continue;
        }
        for member in refs(entity.attributes.get(slot::ASSIGNS_RELATED)) {
            if model.get(member).is_none() {
                anomalies.push(SystemAnomaly::Dangling {
                    relation,
                    missing: member,
                });
                continue;
            }
            members.entry(group).or_default().push(member);
        }
    }

    // `ids_of_type` is an EXACT index: asking it for IFCSYSTEM misses every
    // IfcDistributionSystem in the file, which is the common case. Systems are
    // therefore selected by schema ancestry over the file's own type keys.
    let mut systems = Vec::new();
    for id in system_ids(model, release) {
        let Some(entity) = model.get(id) else {
            continue;
        };
        // Upper-cased for the same reason as `NotASystem::type_name`.
        let type_name = entity.type_name.to_ascii_uppercase();
        systems.push(System {
            id,
            name: text(model, id, slot::NAME),
            long_name: long_name(model, release, id, &type_name),
            predefined_type: predefined_type(model, release, id, &type_name),
            type_name,
            members: members.remove(&id).unwrap_or_default(),
        });
    }
    Ok((systems, anomalies))
}

/// Ids of every entity whose declared type is `IfcSystem` or a subtype,
/// under `release`.
///
/// Deliberately not `Model::ids_of_type`, which indexes the exact type name
/// only: a file whose systems are all `IfcDistributionSystem` would return
/// nothing and the crate would report a model with no systems at all.
fn system_ids(model: &Model, release: Release) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (type_name, _) in model.type_histogram() {
        if release.is_a(type_name, "IFCSYSTEM") {
            out.extend_from_slice(model.ids_of_type(type_name));
        }
    }
    out.sort_unstable();
    out
}

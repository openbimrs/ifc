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

use ifc_model::{EntityId, Model, Value};

use crate::error::SystemAnomaly;
use crate::release::{self, Release};

/// Attribute slots, named so a misread is a compile error rather than a
/// silently empty result.
pub(crate) mod slot {
    /// `IfcRelAssignsToGroup.RelatedObjects`.
    pub const ASSIGNS_RELATED: usize = 4;
    /// `IfcRelAssignsToGroup.RelatingGroup` -- 6, not 5.
    pub const ASSIGNS_GROUP: usize = 6;
}

/// A system as the file states it, with its members resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    /// The `IfcSystem` (or subtype) entity.
    pub id: EntityId,
    /// Declared type, e.g. `IFCDISTRIBUTIONSYSTEM`.
    pub type_name: String,
    /// `Name`, when present.
    pub name: Option<String>,
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
/// Reads against the release the model's `FILE_SCHEMA` header declares
/// (IFC2X3 or IFC4). A model that declares neither -- including every
/// hand-built model in this crate's own tests, which set no header at all
/// -- reads as IFC4, matching this function's pre-#52 behaviour. A model
/// that explicitly declares an unsupported release (e.g. IFC4X3) also
/// falls back to IFC4 here, because this function's `(Vec<_>, Vec<_>)`
/// signature has no slot for a hard refusal; call [`crate::schema_of`]
/// first if that distinction matters to the caller.
pub fn systems(model: &Model) -> (Vec<System>, Vec<SystemAnomaly>) {
    let release = release::resolve_or_ifc4(model);
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
        systems.push(System {
            id,
            // Upper-cased for the same reason as `NotASystem::type_name`.
            type_name: entity.type_name.to_ascii_uppercase(),
            name: text(model, id, 2),
            members: members.remove(&id).unwrap_or_default(),
        });
    }
    (systems, anomalies)
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

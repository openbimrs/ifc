//! What a system serves: `IfcRelServicesBuildings` and, in IFC4X3, the
//! `ServicesFacilities` references (#230).
//!
//! # The schema, per release
//!
//! ```text
//! IFC2X3_TC1   IfcRelServicesBuildings  RelatingSystem : IfcSystem;
//!                                       RelatedBuildings : SET [1:?] OF IfcSpatialStructureElement;
//! IFC4         IfcRelServicesBuildings  RelatedBuildings : SET [1:?] OF IfcSpatialElement;
//! IFC4X3_ADD2  IfcRelServicesBuildings  RelatedBuildings : SET [1:?] OF IfcSpatialElement;
//!
//! IfcSystem    ServicesBuildings  : SET [0:1] OF IfcRelServicesBuildings FOR RelatingSystem;
//! IFC4X3_ADD2  ServicesFacilities : SET [0:?] OF IfcRelReferencedInSpatialStructure FOR RelatedElements;
//! ```
//!
//! The admissible target type is taken from the declared release's table
//! (`RelatedBuildings`' element type), never assumed: an `IfcSpatialZone`
//! is a legal target in IFC4 and IFC4X3 and not an entity at all in IFC2X3.
//!
//! `ServicesBuildings` is `SET [0:1]`: a system states at most one such
//! relationship. A second is kept out and reported as
//! [`SystemAnomaly::ServicesBuildingsTwice`]; the lowest relationship id
//! wins so the result is deterministic.
//!
//! `ServicesFacilities` exists only where the release lets
//! `IfcRelReferencedInSpatialStructure.RelatedElements` hold a group
//! (`IfcSpatialReferenceSelect` in IFC4X3; `IfcProduct` in IFC2X3 and
//! IFC4). That is decided from the table, so under IFC2X3 and IFC4 no
//! facility is read for a system; such a reference stays visible through
//! [`crate::spatial_placements`].

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};

use crate::error::SystemAnomaly;
use crate::release::Release;
use crate::zone::spatial_group::slot as placement;

/// Attribute slots of `IfcRelServicesBuildings`, pinned per release in
/// `release.rs`.
pub(crate) mod slot {
    /// `IfcRelServicesBuildings.RelatingSystem`.
    pub const RELATING_SYSTEM: usize = 4;
    /// `IfcRelServicesBuildings.RelatedBuildings`.
    pub const RELATED_BUILDINGS: usize = 5;
}

pub(crate) const SERVICES_BUILDINGS: &str = "IFCRELSERVICESBUILDINGS";
const REFERENCED: &str = "IFCRELREFERENCEDINSPATIALSTRUCTURE";

/// The element type `entity.attribute` admits under `release`, from its
/// table: the innermost type of an aggregate.
pub(crate) fn declared_type(
    release: Release,
    entity: &str,
    attribute: &str,
) -> Option<&'static str> {
    release
        .schema
        .attributes(entity)
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(attribute))
        .map(|a| a.type_name.as_str())
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

fn is_system(model: &Model, release: Release, id: EntityId) -> Option<Result<(), String>> {
    let entity = model.get(id)?;
    let type_name = entity.type_name.to_ascii_uppercase();
    Some(if release.is_a(&type_name, "IFCSYSTEM") {
        Ok(())
    } else {
        Err(type_name)
    })
}

/// The structures each system serves through `IfcRelServicesBuildings`, in
/// the kept relationship's file order, with anomalies appended.
pub(crate) fn serviced_buildings(
    model: &Model,
    release: Release,
    anomalies: &mut Vec<SystemAnomaly>,
) -> BTreeMap<EntityId, Vec<EntityId>> {
    let target = declared_type(release, SERVICES_BUILDINGS, "RelatedBuildings");
    let mut kept: BTreeMap<EntityId, EntityId> = BTreeMap::new();
    let mut out: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    let mut relations = model.ids_of_type(SERVICES_BUILDINGS).to_vec();
    relations.sort_unstable();
    for relation in relations {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let system = match entity.attributes.get(slot::RELATING_SYSTEM) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        match is_system(model, release, system) {
            None => {
                anomalies.push(SystemAnomaly::Dangling {
                    relation,
                    missing: system,
                });
                continue;
            }
            Some(Err(type_name)) => {
                anomalies.push(SystemAnomaly::NotASystem {
                    relation,
                    group: system,
                    type_name,
                });
                continue;
            }
            Some(Ok(())) => {}
        }
        if let Some(&first) = kept.get(&system) {
            anomalies.push(SystemAnomaly::ServicesBuildingsTwice {
                system,
                kept: first,
                rejected: relation,
            });
            continue;
        }
        kept.insert(system, relation);
        let served = out.entry(system).or_default();
        for building in refs(entity.attributes.get(slot::RELATED_BUILDINGS)) {
            let Some(target_entity) = model.get(building) else {
                anomalies.push(SystemAnomaly::Dangling {
                    relation,
                    missing: building,
                });
                continue;
            };
            let type_name = target_entity.type_name.to_ascii_uppercase();
            if !target.is_some_and(|t| release.schema.accepts_type(t, &type_name)) {
                anomalies.push(SystemAnomaly::ServicedNotSpatial {
                    relation,
                    target: building,
                    type_name,
                });
                continue;
            }
            served.push(building);
        }
    }
    out
}

/// The spatial elements referencing each system through
/// `IfcRelReferencedInSpatialStructure` (`IfcSystem.ServicesFacilities`),
/// ascending by id. Empty under a release whose `RelatedElements` cannot
/// hold a system.
pub(crate) fn serviced_facilities(
    model: &Model,
    release: Release,
    anomalies: &mut Vec<SystemAnomaly>,
) -> BTreeMap<EntityId, BTreeSet<EntityId>> {
    let mut out: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
    let admits_system = declared_type(release, REFERENCED, "RelatedElements")
        .is_some_and(|declared| release.schema.accepts_type(declared, "IFCSYSTEM"));
    if !admits_system {
        return out;
    }
    let structure_type = declared_type(release, REFERENCED, "RelatingStructure");
    let mut relations = model.ids_of_type(REFERENCED).to_vec();
    relations.sort_unstable();
    for relation in relations {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let systems: Vec<EntityId> = refs(entity.attributes.get(placement::RELATED_ELEMENTS))
            .into_iter()
            .filter(|&id| matches!(is_system(model, release, id), Some(Ok(()))))
            .collect();
        if systems.is_empty() {
            continue;
        }
        let structure = match entity.attributes.get(placement::RELATING_STRUCTURE) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        let Some(structure_entity) = model.get(structure) else {
            anomalies.push(SystemAnomaly::Dangling {
                relation,
                missing: structure,
            });
            continue;
        };
        let type_name = structure_entity.type_name.to_ascii_uppercase();
        if !structure_type.is_some_and(|t| release.schema.accepts_type(t, &type_name)) {
            anomalies.push(SystemAnomaly::ServicedNotSpatial {
                relation,
                target: structure,
                type_name,
            });
            continue;
        }
        for system in systems {
            out.entry(system).or_default().insert(structure);
        }
    }
    out
}

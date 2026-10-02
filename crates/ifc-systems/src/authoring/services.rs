//! Authoring `IfcRelServicesBuildings`: the structures a system serves
//! (#230).
//!
//! Both writers bind the model's declared release and check, against that
//! release's own table, that the relating end is an `IfcSystem` and every
//! served end is what `RelatedBuildings` admits there
//! (`IfcSpatialStructureElement` in IFC2X3, `IfcSpatialElement` in IFC4 and
//! IFC4X3). `IfcSystem.ServicesBuildings` is `SET [0:1]`, so a system that
//! already has one, in the model or staged on the transaction, is refused.
//! Every check runs before anything is staged.
//!
//! The IFC4X3 `ServicesFacilities` side is an
//! `IfcRelReferencedInSpatialStructure` listing the system; author it with
//! [`reference_in_spatial_structure`](super::reference_in_spatial_structure).

use std::collections::BTreeSet;

use ifc_model::{Edit, EntityId, Model, Transaction, Value};

use super::error::invalid;
use super::release::{bind, projected_type, Release};
use super::{check_related, refs, SystemAuthoringError, SystemAuthoringResult};
use crate::system::services::slot;

const ENTITY: &str = "IFCRELSERVICESBUILDINGS";

/// Stage an `IfcRelServicesBuildings` in `model`'s declared release, with
/// `OwnerHistory` `$`: `system` serves `buildings`.
///
/// IFC2X3 requires `OwnerHistory`, so there this refuses with
/// [`AuthoringRequired`](SystemAuthoringError::AuthoringRequired); use
/// [`serve_buildings_with_owner_history`].
///
/// # Errors
///
/// [`Invalid`](SystemAuthoringError::Invalid) for a malformed GlobalId, an
/// empty or repeating building set, the system among its own buildings, or a
/// system that already services buildings (`ServicesBuildings` is
/// `SET [0:1]`);
/// [`MultipleSchemas`](SystemAuthoringError::MultipleSchemas) or
/// [`UnsupportedSchema`](SystemAuthoringError::UnsupportedSchema) when the
/// header binds no single verified release;
/// [`MissingReference`](SystemAuthoringError::MissingReference) for a system
/// or building neither in the model nor staged;
/// [`WrongReferenceType`](SystemAuthoringError::WrongReferenceType) for a
/// system that is not an `IfcSystem`, or a building the release's
/// `RelatedBuildings` does not admit. Nothing is staged on an error.
pub fn serve_buildings(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
) -> SystemAuthoringResult<EntityId> {
    serve(tx, model, global_id, system, buildings, None)
}

/// [`serve_buildings`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`serve_buildings`] except the IFC2X3 `AuthoringRequired`, and
/// [`MissingReference`](SystemAuthoringError::MissingReference) or
/// [`WrongReferenceType`](SystemAuthoringError::WrongReferenceType) for an
/// `owner_history` that is absent or not an `IfcOwnerHistory`. Nothing is
/// staged on an error.
pub fn serve_buildings_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
    owner_history: EntityId,
) -> SystemAuthoringResult<EntityId> {
    serve(tx, model, global_id, system, buildings, Some(owner_history))
}

fn serve(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
    owner_history: Option<EntityId>,
) -> SystemAuthoringResult<EntityId> {
    let release = bind(model)?;
    check_related(
        ENTITY,
        "RelatedBuildings",
        global_id,
        system,
        buildings,
        "system",
    )?;
    let mut seen = BTreeSet::new();
    if let Some(repeated) = buildings.iter().find(|id| !seen.insert(**id)) {
        return Err(invalid(
            ENTITY,
            "RelatedBuildings",
            format!("repeats {repeated}"),
        ));
    }
    require(release, tx, model, "RelatingSystem", system)?;
    for &building in buildings {
        require(release, tx, model, "RelatedBuildings", building)?;
    }
    if let Some(existing) = services_of(tx, model, system) {
        return Err(invalid(
            ENTITY,
            "RelatingSystem",
            format!("{system} already services buildings through {existing}"),
        ));
    }
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingSystem", Value::Ref(system)),
        ("RelatedBuildings", refs(buildings)),
    ];
    match owner_history {
        Some(owner_history) => release.stage(tx, model, ENTITY, values, owner_history),
        None => {
            let record = release.record(ENTITY, values)?;
            Ok(tx.create(record))
        }
    }
}

/// Fail unless `target` exists (in the model or staged) with a type the
/// release's `IfcRelServicesBuildings.attribute` admits.
fn require(
    release: Release,
    tx: &Transaction,
    model: &Model,
    attribute: &'static str,
    target: EntityId,
) -> SystemAuthoringResult<()> {
    let actual =
        projected_type(tx, model, target).ok_or(SystemAuthoringError::MissingReference {
            entity: ENTITY,
            attribute,
            target,
        })?;
    let schema = release.schema();
    let declared = schema
        .attributes(ENTITY)
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(attribute))
        .map(|a| a.type_name.as_str())
        .ok_or(SystemAuthoringError::EntityNotInSchema {
            entity: ENTITY,
            schema: release.version(),
        })?;
    if schema.accepts_type(declared, &actual) {
        Ok(())
    } else {
        Err(SystemAuthoringError::WrongReferenceType {
            entity: ENTITY,
            attribute,
            target,
            actual: actual.to_ascii_uppercase(),
            expected: declared,
        })
    }
}

/// An `IfcRelServicesBuildings` whose `RelatingSystem` is `system` once `tx`
/// commits, if any.
fn services_of(tx: &Transaction, model: &Model, system: EntityId) -> Option<EntityId> {
    let staged = tx.edits().iter().filter_map(|edit| match edit {
        Edit::Create { id, .. } => Some(*id),
        _ => None,
    });
    let mut candidates: Vec<EntityId> = model.ids_of_type(ENTITY).to_vec();
    candidates.extend(staged);
    candidates.sort_unstable();
    candidates.dedup();
    candidates.into_iter().find(|&id| {
        projected_type(tx, model, id).is_some_and(|t| t.eq_ignore_ascii_case(ENTITY))
            && projected_system(tx, model, id) == Some(system)
    })
}

/// `RelatingSystem` of relationship `id` once `tx` commits.
fn projected_system(tx: &Transaction, model: &Model, id: EntityId) -> Option<EntityId> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Remove { id: removed } if *removed == id => return None,
            Edit::SetAttribute {
                id: edited,
                slot: at,
                value,
            } if *edited == id && *at == slot::RELATING_SYSTEM => {
                return match value {
                    Value::Ref(system) => Some(*system),
                    _ => None,
                }
            }
            Edit::Create {
                id: created,
                entity,
            } if *created == id => {
                return match entity.attributes.get(slot::RELATING_SYSTEM) {
                    Some(Value::Ref(system)) => Some(*system),
                    _ => None,
                }
            }
            _ => {}
        }
    }
    match model.get(id)?.attributes.get(slot::RELATING_SYSTEM)? {
        Value::Ref(system) => Some(*system),
        _ => None,
    }
}

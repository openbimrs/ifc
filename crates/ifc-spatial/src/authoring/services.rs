//! `IfcRelServicesBuildings`: the spatial elements a system serves (#286).
//!
//! From the EXPRESS sources of every verified release:
//!
//! ```text
//! IFC2X3_TC1   RelatingSystem : IfcSystem;
//!              RelatedBuildings : SET [1:?] OF IfcSpatialStructureElement;
//! IFC4         RelatingSystem : IfcSystem;
//!              RelatedBuildings : SET [1:?] OF IfcSpatialElement;
//! IFC4X3_ADD2  RelatingSystem : IfcSystem;
//!              RelatedBuildings : SET [1:?] OF IfcSpatialElement;
//! IfcSystem    INVERSE ServicesBuildings : SET [0:1] OF
//!              IfcRelServicesBuildings FOR RelatingSystem;
//! ```
//!
//! Both writers bind the model's declared release and check each end
//! against that release's own table, refuse an empty or repeating set, and
//! refuse a second relationship for a system that already has one, in the
//! model or staged on the transaction. Every check runs before anything is
//! staged.
//!
//! `ifc-systems` authors the same relationship with the same rules
//! (`ifc_systems::serve_buildings`, #277). Sibling domain crates do not
//! depend on one another, so the rules are stated here from the schema
//! sources rather than shared; `tests/services_buildings.rs` checks every
//! refusal in each release.

use std::collections::BTreeSet;

use ifc_model::{Edit, EntityId, Model, Transaction, Value};

use super::check_relate;
use super::owned_relationships::refs;
use super::release::{bind, projected_type, stage, Release};
use super::{invalid, SpatialAuthoringError, SpatialAuthoringResult};
use crate::relation::slots::SERVICES_BUILDINGS;

const ENTITY: &str = SERVICES_BUILDINGS.type_name;

/// Stage an `IfcRelServicesBuildings` in `model`'s declared release, with
/// `OwnerHistory` `$`: `system` serves `buildings`.
///
/// IFC2X3 requires `OwnerHistory`, so there this refuses with
/// [`AuthoringRequired`](SpatialAuthoringError::AuthoringRequired); use
/// [`serve_buildings_with_owner_history`].
///
/// # Errors
///
/// [`Invalid`](SpatialAuthoringError::Invalid) for a malformed GlobalId, an
/// empty or repeating building set, the system among its own buildings, or
/// a system that already services buildings (`ServicesBuildings` is
/// `SET [0:1]`);
/// [`MultipleSchemas`](SpatialAuthoringError::MultipleSchemas) or
/// [`UnsupportedSchema`](SpatialAuthoringError::UnsupportedSchema) when the
/// header binds no single verified release;
/// [`MissingReference`](SpatialAuthoringError::MissingReference) for a
/// system or building neither in the model nor staged;
/// [`WrongReferenceType`](SpatialAuthoringError::WrongReferenceType) for a
/// system that is not an `IfcSystem`, or a building the release's
/// `RelatedBuildings` does not admit. Nothing is staged on an error.
pub fn serve_buildings(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    serve(tx, model, global_id, system, buildings, None)
}

/// [`serve_buildings`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`serve_buildings`] except the IFC2X3 `AuthoringRequired`, and
/// [`MissingReference`](SpatialAuthoringError::MissingReference) or
/// [`WrongReferenceType`](SpatialAuthoringError::WrongReferenceType) for an
/// `owner_history` that is absent or not an `IfcOwnerHistory`. Nothing is
/// staged on an error.
pub fn serve_buildings_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
    owner_history: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    serve(tx, model, global_id, system, buildings, Some(owner_history))
}

fn serve(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    system: EntityId,
    buildings: &[EntityId],
    owner_history: Option<EntityId>,
) -> SpatialAuthoringResult<EntityId> {
    let release = bind(model)?;
    check_relate(SERVICES_BUILDINGS, global_id, system, buildings)?;
    let mut seen = BTreeSet::new();
    if let Some(repeated) = buildings.iter().find(|id| !seen.insert(**id)) {
        return Err(invalid(
            ENTITY,
            "RelatedBuildings",
            format!("#{} repeats in a SET", repeated.0),
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
            format!(
                "#{} already services buildings through #{}",
                system.0, existing.0
            ),
        ));
    }
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("RelatingSystem", Value::Ref(system)),
        ("RelatedBuildings", refs(buildings)),
    ];
    stage(tx, model, ENTITY, values, owner_history)
}

/// Fail unless `target` exists (in the model or staged) with a type the
/// release's `IfcRelServicesBuildings.attribute` admits.
fn require(
    release: Release,
    tx: &Transaction,
    model: &Model,
    attribute: &'static str,
    target: EntityId,
) -> SpatialAuthoringResult<()> {
    let actual = projected_type(tx, model, target)
        .ok_or(SpatialAuthoringError::MissingReference {
            entity: ENTITY,
            attribute,
            target,
        })?
        .to_ascii_uppercase();
    let schema = release.schema();
    let declared = schema
        .attributes(ENTITY)
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(attribute))
        .map(|a| a.type_name.as_str())
        .ok_or(SpatialAuthoringError::EntityNotInSchema {
            entity: ENTITY,
            schema: release.version(),
        })?;
    if schema.accepts_type(declared, &actual) {
        Ok(())
    } else {
        Err(SpatialAuthoringError::WrongReferenceType {
            entity: ENTITY,
            attribute,
            target,
            actual,
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
    let slot = SERVICES_BUILDINGS.relating;
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Remove { id: removed } if *removed == id => return None,
            Edit::SetAttribute {
                id: edited,
                slot: at,
                value,
            } if *edited == id && *at == slot => {
                return match value {
                    Value::Ref(system) => Some(*system),
                    _ => None,
                }
            }
            Edit::Create {
                id: created,
                entity,
            } if *created == id => {
                return match entity.attributes.get(slot) {
                    Some(Value::Ref(system)) => Some(*system),
                    _ => None,
                }
            }
            _ => {}
        }
    }
    match model.get(id)?.attributes.get(slot)? {
        Value::Ref(system) => Some(*system),
        _ => None,
    }
}

//! Objectified relationship access.
//!
//! IFC does not store parent/child pointers on entities. A wall does not know
//! its storey; an `IfcRelContainedInSpatialStructure` entity names both. So
//! every containment question is a question about relationship entities, and
//! this module reads them.
//!
//! ## Internal split
//!
//! - `slots.rs`: schema-fixed attribute positions for the `IfcRel*` types used here.
//! - `link.rs`: reading a relationship's relating/related ends.
//! - `boundary.rs`: space boundaries, which carry their own attributes
//!   beyond the two ends and so do not fit the generic `Relationship` shape.

pub mod boundary;
mod index;
mod link;
mod slots;

pub use boundary::{BoundaryExposure, BoundaryPhysicality, SpaceBoundary};
pub use index::RelationshipIndex;
pub use link::{Relationship, RelationshipKind};

use ifc_model::{EntityId, Model};

/// Every aggregation, containment, nesting and covering relationship.
///
/// Found through the type index, so cost is proportional to the number of
/// relationships rather than to model size.
#[must_use]
pub fn all(model: &Model) -> Vec<Relationship> {
    let mut out = Vec::new();
    for (slots, kind) in [
        (slots::AGGREGATES, RelationshipKind::Aggregates),
        (slots::CONTAINED_IN, RelationshipKind::ContainedIn),
        (slots::NESTS, RelationshipKind::Nests),
        (slots::COVERS_ELEMENTS, RelationshipKind::CoversElements),
        (slots::COVERS_SPACES, RelationshipKind::CoversSpaces),
        (
            slots::INTERFERES_ELEMENTS,
            RelationshipKind::InterferesElements,
        ),
        (slots::ASSIGNS_TO_ACTOR, RelationshipKind::AssignsToActor),
        (
            slots::ASSIGNS_TO_PROCESS,
            RelationshipKind::AssignsToProcess,
        ),
        (
            slots::ASSIGNS_TO_PRODUCT,
            RelationshipKind::AssignsToProduct,
        ),
        (
            slots::ASSIGNS_TO_GROUP_BY_FACTOR,
            RelationshipKind::AssignsToGroupByFactor,
        ),
        (slots::DECLARES, RelationshipKind::Declares),
        (slots::DEFINES_BY_OBJECT, RelationshipKind::DefinesByObject),
        (
            slots::FLOW_CONTROL_ELEMENTS,
            RelationshipKind::FlowControlElements,
        ),
        (
            slots::SERVICES_BUILDINGS,
            RelationshipKind::ServicesBuildings,
        ),
        (
            slots::CONNECTS_WITH_ECCENTRICITY,
            RelationshipKind::ConnectsWithEccentricity,
        ),
    ] {
        for id in model.ids_of_type(slots.type_name) {
            if let Some(relationship) = link::read(model, *id, slots, kind) {
                out.push(relationship);
            }
        }
    }
    // The IfcRelConnectsElements hierarchy shares one kind across three
    // concrete types, and its ends sit at 5/6 rather than 4/5 because
    // ConnectionGeometry occupies slot 4.
    for slots in slots::CONNECTS_ELEMENT_TYPES {
        for id in model.ids_of_type(slots.type_name) {
            if let Some(relationship) =
                link::read(model, *id, slots, RelationshipKind::ConnectsElements)
            {
                out.push(relationship);
            }
        }
    }
    out
}

/// The relationships whose related end names `target`.
///
/// The inverse query: given a wall, which containment relationship placed it?
#[must_use]
pub fn naming(model: &Model, target: EntityId) -> Vec<Relationship> {
    all(model)
        .into_iter()
        .filter(|relationship| relationship.related.contains(&target))
        .collect()
}

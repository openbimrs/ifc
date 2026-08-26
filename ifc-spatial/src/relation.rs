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

mod link;
mod slots;

pub use link::{Relationship, RelationshipKind};

use ifc_model::{EntityId, Model};

/// Every aggregation, containment and nesting relationship in the model.
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
    ] {
        for id in model.ids_of_type(slots.type_name) {
            if let Some(relationship) = link::read(model, *id, slots, kind) {
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

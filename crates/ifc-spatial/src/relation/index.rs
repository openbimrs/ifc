//! Reusable inverse lookups for objectified spatial relationships.

use ifc_model::{EntityId, Model, ReverseIndex};

use super::link;
use super::slots;
use super::{Relationship, RelationshipKind};

/// Reverse-reference index paired with the model snapshot it indexes.
///
/// Build this once when querying many entities. It preserves the tolerant
/// decoding of [`super::naming`] while avoiding a relationship rescan per
/// target. Mutating the borrowed model requires dropping this index first, so
/// its derived references cannot silently become stale.
#[derive(Debug)]
pub struct RelationshipIndex<'m> {
    model: &'m Model,
    reverse: ReverseIndex,
}

impl<'m> RelationshipIndex<'m> {
    /// Build one reverse index over the model snapshot.
    #[must_use]
    pub fn build(model: &'m Model) -> Self {
        Self {
            model,
            reverse: ReverseIndex::build(model),
        }
    }

    /// Relationships whose related/member end names `target`.
    ///
    /// Results use the same deterministic kind and entity ordering as
    /// [`super::all`]. Malformed or unrelated records that merely reference the
    /// target from the same absolute slot are ignored.
    #[must_use]
    pub fn naming(&self, target: EntityId) -> Vec<Relationship> {
        let mut out = Vec::new();
        for (rel_slots, kind) in [
            (slots::AGGREGATES, RelationshipKind::Aggregates),
            (slots::CONTAINED_IN, RelationshipKind::ContainedIn),
            (slots::NESTS, RelationshipKind::Nests),
        ] {
            for id in self.reverse.referrers_in_slot(target, rel_slots.related) {
                if let Some(relationship) = link::read(self.model, id, rel_slots, kind) {
                    out.push(relationship);
                }
            }
        }
        out
    }
}

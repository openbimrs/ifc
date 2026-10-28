//! Target-to-referrer index: "which entities point at this one?"
//!
//! # Why this is not built during insertion
//!
//! The type index is built eagerly because almost every consumer asks for
//! "every IfcWall". The reverse index is different: a codec that parses a file
//! and writes it straight back never asks who references what, and paying for
//! the index on every load would tax the common path to serve the rarer one.
//!
//! So it is built on first request, from a `&Model`. [`ReverseIndex::build`]
//! hands back a value the caller owns and can drop: a caller doing one lookup
//! pays for one scan; a caller doing thousands builds it once.
//! [`Model::reverse_index`] builds one the model keeps, and from then on every
//! write updates it (`maintained.rs`), so editing a large model does not
//! rescan it for each edit (#587).
//!
//! # Slots are recorded, not just referrers
//!
//! IFC relationships are objectified: `IfcRelContainedInSpatialStructure` holds
//! its elements in one attribute and its container in another. "Who references
//! this storey" is not enough -- a consumer must know *which slot* the
//! reference sat in to tell containment from the inverse. So each hit carries
//! the attribute index.

use ahash::AHashMap;

use crate::value::EntityId;
use crate::Model;

/// One reference to a target entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Referrer {
    /// The entity holding the reference.
    pub from: EntityId,
    /// Which top-level attribute slot of `from` the reference sits in.
    ///
    /// Nested references inside a `List` or `Typed` wrapper report the index of
    /// the outermost attribute, because that is the slot the schema names.
    pub slot: usize,
}

/// Reverse-reference index over a model snapshot.
///
/// A value from [`ReverseIndex::build`] reflects the model as it was when
/// built: mutating the model afterwards does not update it, and the borrow
/// checker enforces that for the common `&Model` case. The one behind
/// [`Model::reverse_index`] is kept current by the model's writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReverseIndex {
    /// Target to the entities referencing it, sorted and deduplicated so
    /// results are deterministic across runs.
    incoming: AHashMap<EntityId, Vec<Referrer>>,
}

impl ReverseIndex {
    /// Scan every entity once and record each reference by target.
    #[must_use]
    pub fn build(model: &Model) -> Self {
        let mut incoming: AHashMap<EntityId, Vec<Referrer>> = AHashMap::new();
        // File order, so the result is stable rather than hash-ordered.
        for from in model.ids() {
            let Some(entity) = model.get(from) else {
                continue;
            };
            for (slot, attribute) in entity.attributes.iter().enumerate() {
                attribute.for_each_ref(&mut |target| {
                    incoming
                        .entry(target)
                        .or_default()
                        .push(Referrer { from, slot });
                });
            }
        }
        // An attribute may name the same target twice (a degenerate polyline
        // closing on its start point). Report the pair once per slot.
        for referrers in incoming.values_mut() {
            referrers.sort_unstable();
            referrers.dedup();
        }
        Self { incoming }
    }

    /// Records `referrer` of `target`, keeping the ascending order and the
    /// one-pair-per-slot rule of [`ReverseIndex::build`].
    pub(crate) fn link(&mut self, target: EntityId, referrer: Referrer) {
        let referrers = self.incoming.entry(target).or_default();
        if let Err(at) = referrers.binary_search(&referrer) {
            referrers.insert(at, referrer);
        }
    }

    /// Forgets `referrer` of `target`, and `target` once nothing refers to
    /// it.
    pub(crate) fn unlink(&mut self, target: EntityId, referrer: Referrer) {
        let Some(referrers) = self.incoming.get_mut(&target) else {
            return;
        };
        if let Ok(at) = referrers.binary_search(&referrer) {
            referrers.remove(at);
        }
        if referrers.is_empty() {
            self.incoming.remove(&target);
        }
    }

    /// Every reference pointing at `target`, in ascending `(from, slot)` order.
    #[must_use]
    pub fn referrers(&self, target: EntityId) -> &[Referrer] {
        self.incoming.get(&target).map_or(&[], Vec::as_slice)
    }

    /// Entities referencing `target` from the given attribute slot.
    ///
    /// This is the query objectified relationships need: "which relationship
    /// entities name me in their *RelatingStructure* slot".
    pub fn referrers_in_slot(
        &self,
        target: EntityId,
        slot: usize,
    ) -> impl Iterator<Item = EntityId> + '_ {
        self.referrers(target)
            .iter()
            .filter(move |r| r.slot == slot)
            .map(|r| r.from)
    }

    /// Whether anything references `target`.
    #[must_use]
    pub fn is_referenced(&self, target: EntityId) -> bool {
        !self.referrers(target).is_empty()
    }

    /// Number of distinct targets that have at least one referrer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.incoming.len()
    }

    /// Whether no entity references any other.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.incoming.is_empty()
    }
}

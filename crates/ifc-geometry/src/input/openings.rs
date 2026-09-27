//! Which openings void a host element (`IfcRelVoidsElement`).
//!
//! # Why this is geometry input and not a spatial query
//!
//! `ifc-spatial` indexes the relation for navigation, but only this crate
//! needs it to decide what a product's NET shape is: the Body representation
//! of a wall is the gross wall, and the file never authors the net one. So
//! the relation is read here, as a kernel-free slot read, and `lower` turns it
//! into boolean differences.
//!
//! # The slots are stable
//!
//! `RelatingBuildingElement` is slot 4 and `RelatedOpeningElement` slot 5 in
//! IFC2X3 (under `IfcRelConnects`), IFC4 and IFC4X3 (under
//! `IfcRelDecomposes`): both supertypes contribute exactly the four
//! `IfcRoot` attributes. `tests/context_slots.rs` asserts that against the
//! shipped schemas, following ADR 0008.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, Value};

use crate::slots::Slots;

/// Absolute slots on `IfcRelVoidsElement`.
pub mod slot {
    /// `RelatingBuildingElement`: the host that owns the void.
    pub const RELATING_BUILDING_ELEMENT: usize = 4;
    /// `RelatedOpeningElement`: the `IfcFeatureElementSubtraction` removed.
    pub const RELATED_OPENING_ELEMENT: usize = 5;
}

/// The type name the relation is stored under. It has no subtypes.
const VOIDS_ELEMENT: &str = "IFCRELVOIDSELEMENT";

/// One voiding: the relation entity and the opening it names.
#[cfg_attr(not(feature = "lowering"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Voiding {
    /// The `IfcRelVoidsElement` that authored the subtraction.
    pub(crate) relation: EntityId,
    /// The opening removed from the host.
    pub(crate) opening: EntityId,
}

/// An opening that the file makes void a second host.
///
/// `IfcFeatureElementSubtraction.VoidsElements` is a single-valued inverse
/// (`IfcRelVoidsElement FOR RelatedOpeningElement`) in IFC2X3 and IFC4: an
/// opening voids exactly one element. When a file names two hosts, the
/// relation with the lower id wins. The opening is subtracted from that host
/// only, never cut into an element it does not belong to, and the rejected
/// claim is reported here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct VoidingConflict {
    /// The opening with two hosts.
    pub opening: EntityId,
    /// The host it voids.
    pub kept_host: EntityId,
    /// The host it is not subtracted from.
    pub rejected_host: EntityId,
    /// The `IfcRelVoidsElement` that was rejected.
    pub relation: EntityId,
}

/// Every well-formed `(relation, host, opening)` triple, ascending by relation.
fn all_voidings(model: &Model) -> Vec<(EntityId, EntityId, EntityId)> {
    let mut all: Vec<_> = model
        .of_type(VOIDS_ELEMENT)
        .filter_map(|(relation, entity)| {
            let slots = Slots::new(relation, entity);
            let host = ref_at(slots.opt(slot::RELATING_BUILDING_ELEMENT))?;
            let opening = ref_at(slots.opt(slot::RELATED_OPENING_ELEMENT))?;
            Some((relation, host, opening))
        })
        .collect();
    all.sort_unstable();
    all
}

/// Each opening's host: the one its lowest-id relation names.
fn kept_hosts(all: &[(EntityId, EntityId, EntityId)]) -> BTreeMap<EntityId, EntityId> {
    let mut kept = BTreeMap::new();
    for &(_, host, opening) in all {
        kept.entry(opening).or_insert(host);
    }
    kept
}

/// Openings the file makes void more than one host, in relation order.
///
/// Empty for a conformant file. Kernel-free, like [`openings_of`]. A second
/// relation naming the same host again is redundant, not a conflict.
#[must_use]
pub fn voiding_conflicts(model: &Model) -> Vec<VoidingConflict> {
    let all = all_voidings(model);
    let kept = kept_hosts(&all);
    all.iter()
        .filter(|(_, host, opening)| kept[opening] != *host)
        .map(|&(relation, host, opening)| VoidingConflict {
            opening,
            kept_host: kept[&opening],
            rejected_host: host,
            relation,
        })
        .collect()
}

/// Every voiding of `host`, one per opening, in ascending opening id.
///
/// Ordered by the opening rather than by the relation so the result is a
/// property of the model's content, not of which relation an exporter wrote
/// first. An opening named by two relations for the same host is subtracted
/// once: removing the same body twice changes nothing but the cost. An
/// opening another relation assigns to a different host first is not this
/// host's (see [`voiding_conflicts`]).
///
/// Scans the relations on each call. That is O(relations), which is a few
/// hundred on a real building; a caller netting every product of a large model
/// pays O(products x relations), still well under the boolean cost.
pub(crate) fn voidings_of(model: &Model, host: EntityId) -> Vec<Voiding> {
    let all = all_voidings(model);
    let kept = kept_hosts(&all);
    let mut found: Vec<Voiding> = all
        .into_iter()
        .filter(|(_, relating, opening)| *relating == host && kept[opening] == host)
        .map(|(relation, _, opening)| Voiding { relation, opening })
        .collect();
    found.sort_by_key(|voiding| (voiding.opening, voiding.relation));
    found.dedup_by_key(|voiding| voiding.opening);
    found
}

/// The openings voiding `host`, in ascending id.
///
/// Kernel-free: answering which openings a wall has is a slot read, so this
/// works under `--no-default-features` exactly as with the kernel linked. A
/// relation with a missing or non-reference end is skipped rather than
/// guessed at; it names no opening this function could return. An opening a
/// malformed file assigns to two hosts belongs to the first by relation id;
/// [`voiding_conflicts`] reports the other.
///
/// ```
/// use ifc_geometry::openings_of;
/// use ifc_model::{EntityId, Model};
///
/// // A model with no relations has no openings anywhere.
/// assert!(openings_of(&Model::new(), EntityId(1)).is_empty());
/// ```
pub fn openings_of(model: &Model, host: EntityId) -> Vec<EntityId> {
    voidings_of(model, host)
        .into_iter()
        .map(|voiding| voiding.opening)
        .collect()
}

fn ref_at(value: Option<&Value>) -> Option<EntityId> {
    match value? {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

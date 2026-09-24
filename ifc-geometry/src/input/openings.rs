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

/// Every voiding of `host`, one per opening, in ascending opening id.
///
/// Ordered by the opening rather than by the relation so the result is a
/// property of the model's content, not of which relation an exporter wrote
/// first. An opening named by two relations is subtracted once: removing the
/// same body twice changes nothing but the cost.
///
/// Scans the relations on each call. That is O(relations), which is a few
/// hundred on a real building; a caller netting every product of a large model
/// pays O(products x relations), still well under the boolean cost.
pub(crate) fn voidings_of(model: &Model, host: EntityId) -> Vec<Voiding> {
    let mut found: Vec<Voiding> = model
        .of_type(VOIDS_ELEMENT)
        .filter_map(|(relation, entity)| {
            let slots = Slots::new(relation, entity);
            let relating = ref_at(slots.opt(slot::RELATING_BUILDING_ELEMENT))?;
            let opening = ref_at(slots.opt(slot::RELATED_OPENING_ELEMENT))?;
            (relating == host).then_some(Voiding { relation, opening })
        })
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
/// guessed at; it names no opening this function could return.
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

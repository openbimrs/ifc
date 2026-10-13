//! What a cascading removal or a purge did, and what it left in place.

use ifc_model::EntityId;

/// What [`AuthorOp::RemoveProduct`](super::AuthorOp::RemoveProduct) or
/// [`AuthorOp::Purge`](super::AuthorOp::Purge) removed, and what it kept
/// and why. Every list is in id order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemovalReport {
    /// Every entity removed: the product and what it solely owned, or the
    /// purged entities, with the relationships left without an end.
    pub removed: Vec<RemovedEntity>,
    /// Entities the cascade reached and kept, each with its reason.
    pub kept: Vec<KeptEntity>,
    /// Relationships and presentation layer assignments that lost a
    /// removed member from a list and stay, relating the rest.
    pub detached: Vec<EntityId>,
    /// Placements that were relative to a removed placement, re-based onto
    /// the nearest kept ancestor with the composed transform
    /// ([`RemoveProductOptions::rebase_placements`](super::RemoveProductOptions::rebase_placements)).
    pub rebased: Vec<EntityId>,
}

/// One removed entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RemovedEntity {
    /// Its id.
    pub id: EntityId,
    /// Its type, as the file writes it (`IFCPROPERTYSET`).
    pub type_name: String,
}

/// One entity the cascade reached and kept.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeptEntity {
    /// Its id.
    pub id: EntityId,
    /// Its type, as the file writes it.
    pub type_name: String,
    /// Why it stays.
    pub reason: KeepReason,
    /// For [`KeepReason::Shared`], the entities that still use it and
    /// stay; empty otherwise.
    pub users: Vec<EntityId>,
}

/// Why a reached entity was kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeepReason {
    /// Another entity that stays still needs it: a property set another
    /// object is defined by, a type's representation map an occurrence
    /// maps, a style another item is styled with, a point another
    /// placement locates.
    Shared,
    /// An object with an identity of its own, related to a removed one: a
    /// type, a container, a filling of a removed opening, a part of a
    /// removed whole. Removing it is its own decision.
    RelatedObject,
    /// A definition the cascade never deletes, used or not: a material
    /// definition, a classification, document or library reference, a
    /// unit, a representation context, the owner history and its actors.
    /// [`AuthorOp::Purge`](super::AuthorOp::Purge) removes them once
    /// nothing references them.
    Definition,
    /// Purge only: unreferenced, but of a kind purge removes only when
    /// asked ([`PurgeOptions`](super::PurgeOptions)): an `IfcRoot`, the
    /// owner history and its actors, or a representation context.
    Protected,
}

impl KeepReason {
    /// The reason's name, as the bindings spell it: `shared`,
    /// `related-object`, `definition`, `protected`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::RelatedObject => "related-object",
            Self::Definition => "definition",
            Self::Protected => "protected",
        }
    }
}

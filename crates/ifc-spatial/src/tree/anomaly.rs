//! Conflicts the file states and the tree had to resolve.

use ifc_model::EntityId;

/// A statement of the file the tree could not honour, recorded instead of
/// silently dropped.
///
/// A second parent the schema forbids: both
/// `IfcElement.ContainedInStructure` and `IfcObjectDefinition.Decomposes`
/// are `SET [0:1]`. A file naming two parents is malformed; the first
/// relationship processed wins so the tree stays a tree, and the loser is
/// reported here. Every view of the tree agrees with the kept answer.
///
/// A spatial relationship whose structure is not a spatial container of the
/// release: its elements have nowhere to be listed, so the relationship is
/// reported rather than ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpatialAnomaly {
    /// An element placed in two different spatial structures.
    ContainedTwice {
        /// The element with two homes.
        element: EntityId,
        /// The structure kept.
        kept: EntityId,
        /// The structure rejected; it does not list the element.
        rejected: EntityId,
        /// The `IfcRelContainedInSpatialStructure` that was rejected.
        relation: EntityId,
    },
    /// A container aggregated by two different parents.
    AggregatedTwice {
        /// The container with two parents.
        child: EntityId,
        /// The parent kept.
        kept: EntityId,
        /// The parent rejected; it does not list the child.
        rejected: EntityId,
        /// The `IfcRelAggregates` that was rejected.
        relation: EntityId,
    },
    /// An `IfcRelContainedInSpatialStructure` whose `RelatingStructure` is
    /// not a spatial container of the release; its elements are not placed.
    ContainedInNonContainer {
        /// The relationship.
        relation: EntityId,
        /// The entity it names as the structure.
        structure: EntityId,
    },
    /// An `IfcRelReferencedInSpatialStructure` whose `RelatingStructure` is
    /// not a spatial container of the release; its elements are not listed.
    ReferencedInNonContainer {
        /// The relationship.
        relation: EntityId,
        /// The entity it names as the structure.
        structure: EntityId,
    },
}

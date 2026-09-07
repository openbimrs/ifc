//! Attribute positions for the objectified relationships this crate reads.
//!
//! # Why these are constants and not schema lookups
//!
//! These six-slot layouts are fixed across IFC2x3, IFC4 and IFC4x3: the
//! relationship types inherit four attributes from `IfcRoot` and add two of
//! their own. Reading them from the schema would make every traversal depend on
//! a parsed `.exp` file to answer a question whose answer cannot change without
//! a new major schema.
//!
//! # The trap these encode
//!
//! The two relationships this crate cares about **disagree on slot order**:
//!
//! ```text
//! IfcRelAggregates                    4 = RelatingObject   5 = RelatedObjects
//! IfcRelContainedInSpatialStructure   4 = RelatedElements  5 = RelatingStructure
//! ```
//!
//! Assuming a uniform "relating first" layout silently inverts containment:
//! elements become the parents of their storey. The positions below were read
//! from IFC4 ADD2 TC1 and are asserted against the shipped schema in
//! `tests/slot_layout.rs`.

/// Slot layout of one objectified relationship.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RelSlots {
    /// STEP type name, upper-case as stored.
    pub type_name: &'static str,
    /// Slot holding the parent/owner end.
    pub relating: usize,
    /// Slot holding the child/member end.
    pub related: usize,
}

/// `IfcRelAggregates`: decomposition, e.g. site to building, building to storey.
pub(crate) const AGGREGATES: RelSlots = RelSlots {
    type_name: "IFCRELAGGREGATES",
    relating: 4,
    related: 5,
};

/// `IfcRelContainedInSpatialStructure`: elements placed in a spatial container.
///
/// Note the inverted order relative to `AGGREGATES`.
pub(crate) const CONTAINED_IN: RelSlots = RelSlots {
    type_name: "IFCRELCONTAINEDINSPATIALSTRUCTURE",
    relating: 5,
    related: 4,
};

/// `IfcRelNests`: ordered decomposition, e.g. a stair into its flights.
pub(crate) const NESTS: RelSlots = RelSlots {
    type_name: "IFCRELNESTS",
    relating: 4,
    related: 5,
};

/// `IfcRelSpaceBoundary`: which element bounds a space, and how.
///
/// Slot 4 is `RelatingSpace`, slot 5 `RelatedBuildingElement`, so this
/// follows the same "relating first" order as `AGGREGATES`.
pub(crate) const SPACE_BOUNDARY: RelSlots = RelSlots {
    type_name: "IFCRELSPACEBOUNDARY",
    relating: 4,
    related: 5,
};

/// `IfcRelSpaceBoundary1stLevel`: adds `ParentBoundary` at slot 9.
pub(crate) const SPACE_BOUNDARY_1ST: RelSlots = RelSlots {
    type_name: "IFCRELSPACEBOUNDARY1STLEVEL",
    relating: 4,
    related: 5,
};

/// `IfcRelSpaceBoundary2ndLevel`: adds `CorrespondingBoundary` at slot 10.
pub(crate) const SPACE_BOUNDARY_2ND: RelSlots = RelSlots {
    type_name: "IFCRELSPACEBOUNDARY2NDLEVEL",
    relating: 4,
    related: 5,
};

/// Every concrete type in the `IfcRelSpaceBoundary` hierarchy.
///
/// `Model::ids_of_type` matches an EXACT type name: it does not resolve
/// subtypes. A file storing `IfcRelSpaceBoundary2ndLevel` -- which is what
/// every real second-level BEM export writes -- would be invisible to a
/// lookup of the supertype alone, and the crate would report a building
/// with no boundaries at all rather than failing.
///
/// The hierarchy is closed at these three in IFC4, so it is enumerated here
/// and asserted against the shipped schemas in `tests/slot_layout.rs`. This
/// crate deliberately does not depend on `ifc-schema` (see AGENTS.md), so
/// there is no runtime `is_a` available to do it instead.
pub(crate) const SPACE_BOUNDARY_TYPES: [RelSlots; 3] =
    [SPACE_BOUNDARY, SPACE_BOUNDARY_1ST, SPACE_BOUNDARY_2ND];

/// `IfcRelCoversBldgElements`: finishes applied to an element.
///
/// A covering is a distinct element from the wall it clads, so a take-off
/// that ignores this double-counts nothing but misses every finish.
pub(crate) const COVERS_ELEMENTS: RelSlots = RelSlots {
    type_name: "IFCRELCOVERSBLDGELEMENTS",
    relating: 4,
    related: 5,
};

/// `IfcRelCoversSpaces`: finishes bounding a space.
///
/// Distinct from `COVERS_ELEMENTS`: the same suspended ceiling can cover a
/// slab (element) and a room (space), and the two answer different
/// questions. Merging them loses which.
pub(crate) const COVERS_SPACES: RelSlots = RelSlots {
    type_name: "IFCRELCOVERSSPACES",
    relating: 4,
    related: 5,
};

/// `IfcRelConnectsElements`: one element connected to another.
///
/// # The slot shift
///
/// This family puts `ConnectionGeometry` FIRST, at slot 4, so its two ends
/// sit at 5 and 6 -- not 4 and 5 like every other relationship in this
/// module. Reading 4/5 here yields the geometry as the relating end and the
/// relating element as the related end: a connection between a shape and a
/// wall, which is silently wrong rather than an error.
pub(crate) const CONNECTS_ELEMENTS: RelSlots = RelSlots {
    type_name: "IFCRELCONNECTSELEMENTS",
    relating: 5,
    related: 6,
};

/// `IfcRelConnectsPathElements`: a connection carrying path priorities.
pub(crate) const CONNECTS_PATH_ELEMENTS: RelSlots = RelSlots {
    type_name: "IFCRELCONNECTSPATHELEMENTS",
    relating: 5,
    related: 6,
};

/// `IfcRelConnectsWithRealizingElements`: a connection realized by others.
pub(crate) const CONNECTS_WITH_REALIZING: RelSlots = RelSlots {
    type_name: "IFCRELCONNECTSWITHREALIZINGELEMENTS",
    relating: 5,
    related: 6,
};

/// Every concrete type in the `IfcRelConnectsElements` hierarchy.
///
/// Same exact-match problem as the space boundaries: a file storing the
/// path-elements form is invisible to a lookup of the supertype.
pub(crate) const CONNECTS_ELEMENT_TYPES: [RelSlots; 3] = [
    CONNECTS_ELEMENTS,
    CONNECTS_PATH_ELEMENTS,
    CONNECTS_WITH_REALIZING,
];

/// `IfcRelInterferesElements`: two elements occupying the same space.
///
/// Note this is NOT a subtype of `IfcRelConnectsElements` and does NOT
/// share its layout: its ends are back at 4 and 5, with the geometry after
/// them. Clash detection reads this.
pub(crate) const INTERFERES_ELEMENTS: RelSlots = RelSlots {
    type_name: "IFCRELINTERFERESELEMENTS",
    relating: 4,
    related: 5,
};

/// `IfcRelAssignsToActor`: who is responsible for an object.
///
/// # The slot-5 trap
///
/// Every `IfcRelAssigns` subtype interposes `RelatedObjectsType` between the
/// two ends:
///
/// ```text
/// 4 = RelatedObjects   5 = RelatedObjectsType   6 = Relating<something>
/// ```
///
/// So the relating end is at **6**, and the related list comes FIRST at 4 --
/// the inverse of the aggregates layout. Slot 5 holds an enumeration, not a
/// reference, so a reader that assumes "relating at 5" silently finds
/// nothing and the assignment disappears without an error.
pub(crate) const ASSIGNS_TO_ACTOR: RelSlots = RelSlots {
    type_name: "IFCRELASSIGNSTOACTOR",
    relating: 6,
    related: 4,
};

/// `IfcRelAssignsToProcess`: which task consumes or produces an object.
pub(crate) const ASSIGNS_TO_PROCESS: RelSlots = RelSlots {
    type_name: "IFCRELASSIGNSTOPROCESS",
    relating: 6,
    related: 4,
};

/// `IfcRelAssignsToProduct`: which product an object is assigned to.
pub(crate) const ASSIGNS_TO_PRODUCT: RelSlots = RelSlots {
    type_name: "IFCRELASSIGNSTOPRODUCT",
    relating: 6,
    related: 4,
};

/// `IfcRelAssignsToGroupByFactor`: group membership carrying a ratio.
///
/// A concrete subtype of `IfcRelAssignsToGroup`, which other crates already
/// read. It is listed here so the type is reachable at all: an exact-name
/// lookup of the supertype does not find it.
pub(crate) const ASSIGNS_TO_GROUP_BY_FACTOR: RelSlots = RelSlots {
    type_name: "IFCRELASSIGNSTOGROUPBYFACTOR",
    relating: 6,
    related: 4,
};

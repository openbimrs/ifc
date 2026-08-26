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

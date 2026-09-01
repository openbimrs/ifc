//! `ifc-spatial` — containment and objectified relationship traversal.
//!
//! # The problem
//!
//! IFC stores no parent pointers. A wall does not name its storey; a separate
//! `IfcRelContainedInSpatialStructure` entity names both ends. Asking "which
//! elements are on this storey" therefore means finding relationship entities
//! and reading the right attribute slot — and the two relationships that build
//! the tree **disagree about which slot is which**:
//!
//! ```text
//! IfcRelAggregates                    4 = RelatingObject   5 = RelatedObjects
//! IfcRelContainedInSpatialStructure   4 = RelatedElements  5 = RelatingStructure
//! ```
//!
//! Assuming a uniform layout inverts containment silently: elements become the
//! parents of their storey, and every downstream answer is wrong in a way no
//! type error catches.
//!
//! # Example
//!
//! ```no_run
//! use ifc_spatial::{SpatialKind, SpatialTree};
//! # let model = ifc_model::Model::new();
//!
//! let tree = SpatialTree::build(&model);
//!
//! for storey in tree.of_kind(SpatialKind::Storey) {
//!     let elements = tree.elements_of(storey.id);
//!     println!("storey {:?} holds {} elements", storey.id, elements.len());
//! }
//! ```
//!
//! # Tolerating real files
//!
//! The canonical hierarchy is project → site → building → storey → element, and
//! real exports deviate: omitted sites, elements hung directly off a building,
//! duplicate storeys, relationships naming entities that are not in the file.
//! The tree records what the file says and reports anomalies through
//! [`SpatialTree::orphans`] and [`SpatialTree::dangling`] rather than asserting
//! the ideal shape or panicking.
//!
//! # Boundaries
//!
//! This crate reads containment. It does not validate it — `ifc-validate` owns
//! WHERE rules and cardinality — and it does not interpret geometry or
//! properties of the elements it groups.

pub mod relation;
mod tree;

pub use relation::{Relationship, RelationshipIndex, RelationshipKind};
pub use tree::{SpatialKind, SpatialNode, SpatialTree};

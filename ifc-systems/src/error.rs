//! Why a system query failed.
//!
//! Reading a system means reading relationship entities that may name
//! entities the file never defines. That is a property of real exports, not
//! a programming error, so it is reported rather than panicked on.

use ifc_model::EntityId;

/// A system membership the file states but cannot support.
///
/// Anomalies are collected instead of rejected: a file with one broken
/// relationship still has a usable system graph, and refusing the whole
/// model would make the crate useless on real exports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemAnomaly {
    /// A relationship names an entity that is not in the file.
    Dangling {
        /// The relationship entity that made the claim.
        relation: EntityId,
        /// The id it named.
        missing: EntityId,
    },
    /// `IfcRelAssignsToGroup` whose `RelatingGroup` is not a system.
    ///
    /// The relationship is shared with every other kind of group, so a
    /// membership may legitimately point at something this crate does not
    /// model. It is recorded rather than silently dropped.
    NotASystem {
        /// The relationship entity.
        relation: EntityId,
        /// The group it named.
        group: EntityId,
        /// The group's declared type, upper-cased.
        type_name: String,
    },
}

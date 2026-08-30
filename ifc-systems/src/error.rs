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
    /// A port is attached to two different elements.
    ///
    /// `IfcPort.ContainedIn` is `SET [0:1]` in the schema, so this cannot be
    /// expressed by a valid file. It happens when an exporter writes both an
    /// `IfcRelNests` and a legacy `IfcRelConnectsPortToElement` that disagree.
    /// The first attachment in file order is kept so the result stays
    /// deterministic, and the conflict is reported rather than hidden.
    PortAttachedTwice {
        /// The port with two owners.
        port: EntityId,
        /// The element that was kept.
        kept: EntityId,
        /// The element that was rejected.
        rejected: EntityId,
    },
    /// A connection names a port that is not an `IfcPort` subtype.
    ///
    /// `IfcRelConnectsPorts` is typed to `IfcPort` in the schema, so this is a
    /// malformed file rather than a modelling choice.
    NotAPort {
        /// The relationship entity.
        relation: EntityId,
        /// The entity it named as a port.
        entity: EntityId,
        /// That entity's declared type, upper-cased.
        type_name: String,
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

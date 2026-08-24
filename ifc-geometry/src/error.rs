//! Why interpreting a geometry entity failed.
//!
//! Every failure names the entity that caused it. A geometry bug in a
//! 500k-entity file is unfindable otherwise, and "returned None" tells you
//! nothing about which of 3,000 walls was malformed.

use ifc_model::EntityId;
use thiserror::Error;

/// The result of interpreting IFC geometry.
pub type GeometryResult<T> = Result<T, GeometryError>;

/// Failures when reading or lowering IFC geometry.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GeometryError {
    /// An entity referenced by an attribute is not in the model.
    #[error("{referrer} references missing entity {missing}")]
    MissingEntity {
        /// The entity holding the dangling reference.
        referrer: EntityId,
        /// The id that does not resolve.
        missing: EntityId,
    },

    /// An attribute slot was empty but the geometry needs it.
    #[error("{entity} ({type_name}) has no {attribute}")]
    MissingAttribute {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Which attribute was required.
        attribute: &'static str,
    },

    /// An attribute held a value of the wrong shape.
    #[error("{entity} ({type_name}).{attribute}: expected {expected}, found {found}")]
    WrongValueKind {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Which attribute.
        attribute: &'static str,
        /// What the schema requires.
        expected: &'static str,
        /// What the file actually contains.
        found: String,
    },

    /// The entity is not the type the caller assumed.
    #[error("{entity} is {actual}, not a {expected}")]
    WrongEntityType {
        /// The offending entity.
        entity: EntityId,
        /// The type it actually has.
        actual: String,
        /// The type family that was required.
        expected: &'static str,
    },

    /// A recognized IFC entity whose interpretation is not implemented.
    ///
    /// Distinct from [`Self::WrongEntityType`]: the file is valid and we simply
    /// do not handle it yet. Never silently substituted with a wrong shape.
    #[error("{type_name} ({entity}) is valid IFC but not yet interpreted: {detail}")]
    Unsupported {
        /// The entity in question.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// What specifically is missing.
        detail: &'static str,
    },

    /// A placement or mapped-item chain refers back to itself.
    ///
    /// The IFC spec pushes cycle prevention to the application layer, so real
    /// files do contain them. Detecting beats overflowing the stack.
    #[error("cyclic {kind} chain through {entity}")]
    CyclicChain {
        /// Where the cycle was detected.
        entity: EntityId,
        /// What kind of chain: `placement`, `mapped item`, ...
        kind: &'static str,
    },

    /// A chain exceeded its depth limit without closing.
    #[error("{kind} chain through {entity} exceeded depth {limit}")]
    ChainTooDeep {
        /// Where the walk gave up.
        entity: EntityId,
        /// What kind of chain.
        kind: &'static str,
        /// The limit that was hit.
        limit: usize,
    },

    /// The geometry is structurally impossible.
    ///
    /// A degenerate direction, a zero-radius circle, a self-referencing
    /// boolean. The file parses; the geometry does not exist.
    #[error("{entity} ({type_name}) is geometrically invalid: {detail}")]
    Degenerate {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Why it cannot be built.
        detail: String,
    },

    /// Units could not be resolved, so coordinates have no defined scale.
    #[error("unit resolution failed: {0}")]
    Units(String),
}

impl GeometryError {
    /// The entity this error is about, when there is one.
    ///
    /// Lets a caller collect failures per element rather than aborting a whole
    /// file for one bad wall.
    pub fn entity(&self) -> Option<EntityId> {
        match self {
            Self::MissingEntity { referrer, .. } => Some(*referrer),
            Self::MissingAttribute { entity, .. }
            | Self::WrongValueKind { entity, .. }
            | Self::WrongEntityType { entity, .. }
            | Self::Unsupported { entity, .. }
            | Self::CyclicChain { entity, .. }
            | Self::ChainTooDeep { entity, .. }
            | Self::Degenerate { entity, .. } => Some(*entity),
            Self::Units(_) => None,
        }
    }

    /// Is this "valid IFC we do not handle yet" rather than a broken file?
    ///
    /// Callers building a viewer usually want to skip and count these, while
    /// treating genuine corruption differently.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_name_the_entity_so_failures_are_locatable() {
        let e = GeometryError::MissingAttribute {
            entity: EntityId(42),
            type_name: "IFCEXTRUDEDAREASOLID".into(),
            attribute: "SweptArea",
        };
        assert_eq!(e.entity(), Some(EntityId(42)));
        assert!(e.to_string().contains("#42"));
        assert!(e.to_string().contains("SweptArea"));
    }

    #[test]
    fn unsupported_is_distinguishable_from_corruption() {
        let unsupported = GeometryError::Unsupported {
            entity: EntityId(1),
            type_name: "IFCSECTIONEDSPINE".into(),
            detail: "spine interpolation",
        };
        let broken = GeometryError::Degenerate {
            entity: EntityId(1),
            type_name: "IFCCIRCLE".into(),
            detail: "zero radius".into(),
        };
        assert!(unsupported.is_unsupported());
        assert!(!broken.is_unsupported());
    }
}
